use chrono::prelude::*;
use std::collections::HashMap;
use std::time::Duration;

// use chrono::Utc;
// use regex::Regex;
// use futures::{StreamExt, TryStream, TryStreamExt};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::{broadcast, RwLock};
use tokio::time;
// use tokio_stream::wrappers;
use uuid::Uuid;

use crate::holo::holo_api::{
    ChannelModelResponse, ErrorOutput, Input, JoinEvent, MessageEvent, MessageModelResponse,
    Output, RequestPacket, ResponsePacket, UserDiscconnectOutput, UserJoinedOutput,
    UserMessageOutput, UserModelResponse,
};
// use crate::holo::holo_errors::{HoloError, Result};
use crate::model::channel::Channel;
use crate::model::message::Message;
use crate::model::session::Session;

const MAX_MESSAGE_BODY_LENGTH: usize = 256;

#[derive(Clone, Copy, Default)]
pub struct HolocasterConfig {
    pub alive_interval: Option<Duration>,
}

pub struct Holocaster {
    alive_interval: Option<Duration>,
    response_sender: broadcast::Sender<ResponsePacket>,
    sessions: RwLock<HashMap<Uuid, Session>>,
    channels: RwLock<Vec<Channel>>,
}

pub struct DefaultHolonetChannel {
    name: String,
    id: Uuid,
}

lazy_static! {
    static ref DEFAULT_HOLONET_CHANNEL: DefaultHolonetChannel = DefaultHolonetChannel {
        name: String::from("holonet"),
        id: Uuid::parse_str("65fe9132-a31f-11eb-bcbc-0242ac130002").unwrap()
    };
}

// This is intended to be stored in an Arc and be leveraged concurrently across all active processes
impl Holocaster {
    pub fn new(config: HolocasterConfig) -> Self {
        // TODO: i think this means we can maintain 16 channels sending messages at once?
        // 16 is what i found in default documents...
        let (response_sender, _) = broadcast::channel(16);

        // Seed the Holonet Default channel
        // TODO: (for now all users will only use this one chat-channel)
        let channel_default: Vec<Channel> = vec![Channel::new(
            DEFAULT_HOLONET_CHANNEL.id,
            &DEFAULT_HOLONET_CHANNEL.name,
            Uuid::nil(),
        )];

        Holocaster {
            alive_interval: config.alive_interval,
            response_sender: response_sender,
            sessions: Default::default(),
            channels: RwLock::new(channel_default),
        }
    }

    // This kicks off the party
    // If we don't get any activity at the end of our alive_interval, then we fire off a Keep alive message, which closes out the channel
    pub async fn run(&self, request_stream: UnboundedReceiver<RequestPacket>) {
        let process_keep_alive_ticker = self.process_keep_alive();

        tokio::select! {
            _ = process_keep_alive_ticker => {
                println!("KEEP ALIVE FAILED!");},
            _ = self.handle_incoming(request_stream) => {
                println!("COMPLETED sending packets!");
            } ,
        }
    }

    async fn handle_incoming(&self, mut request_stream: UnboundedReceiver<RequestPacket>) {
        while let Some(i) = request_stream.recv().await {
            self.handle_message(i).await;
        }
    }

    // Generate a thread-safe listener from the websocket
    pub fn subscribe(&self) -> broadcast::Receiver<ResponsePacket> {
        return self.response_sender.subscribe();
    }

    // Remove user on disconnect
    pub async fn handle_disconnect(&self, session_id: Uuid) {
        if self.sessions.write().await.remove(&session_id).is_some() {
            self.send_except_session_id(
                session_id,
                Output::UserDisconnect(UserDiscconnectOutput::new(session_id)),
            )
            .await;
        }
    }

    // This is where we handle routing messages to the correct Controller!
    // TODO: Session Generates a Message
    async fn handle_message(&self, request_packet: RequestPacket) {
        match request_packet.body {
            Input::Join(body) => self.process_join(request_packet.session_id, body).await,
            Input::Message(body) => self.process_message(request_packet.session_id, body).await,
        }
    }

    // Handle a user joining the stream
    async fn process_join(&self, session_id: Uuid, body: JoinEvent) {
        // TODO: add validation!
        // I don't have validation right now because we are assuming all the data provided by Holonet is good to go!
        println!("processing join event!");

        // Track the client with a session object
        let session = Session::new(session_id, &body.user_name);
        self.sessions
            .write()
            .await
            .insert(session_id, session.clone());
        println!("saved session event!");

        // Send payload of info to the user that just joined
        let channels = self.get_user_channels().await.unwrap();
        let mut output_packet = UserJoinedOutput {
            channels: vec![],
            user: UserModelResponse {
                id: session_id,
                name: String::from(body.user_name),
            },
        };

        // TODO: this needs to be cleaned up
        // sending messages off
        for channel in channels.iter() {
            println!("sending a response to a chanel!");
            output_packet.channels.push(ChannelModelResponse {
                id: channel.id,
                name: channel.name.clone(),
            });
        }

        println!("Notifying session confirmation of join");
        self.send_session_id(session_id, Output::UserJoined(output_packet.clone()))
            .await;
        println!("Notifying all other users confirmation of join");
        self.send_except_session_id(session_id, Output::UserJoined(output_packet))
            .await;
    }

    //  this function is to get the current channel the user is listening to
    // Maybe this should return back an array instead of just one channel?
    // TODO: We only support the channel in the '0' position for now
    // TODO: this is super sloppy
    async fn get_user_channels(&self) -> Option<Vec<Channel>> {
        let channels = self.channels.read().await;

        if channels.len() > 0 {
            // for channel in channels.iter() {}
            return Some(vec![channels[0].clone()]);
        }
        None
    }

    // async fn get_messages_from_channel(&self, channel: Channel) -> Vec<Message> {}

    // Handle a user sending a message to the stream
    async fn process_message(&self, session_id: Uuid, message: MessageEvent) {
        // Verify authentication of the user
        let user = if let Some(user) = self.sessions.read().await.get(&session_id) {
            user.clone()
        } else {
            self.send_error(session_id, ErrorOutput::InvalidSession)
                .await;
            return;
        };

        if message.body.is_empty() || message.body.len() > MAX_MESSAGE_BODY_LENGTH {
            self.send_error(session_id, ErrorOutput::InvalidMessageRequest)
                .await;
            return;
        }

        let message = Message::new(
            Uuid::new_v4(),
            session_id,
            user.clone(),
            &message.body,
            Utc::now(),
        );

        // Send the message to the DB
        let mut channels = self.channels.write().await;
        channels[0].message_add(message.clone());

        let response_packet = UserMessageOutput::new(
            MessageModelResponse {
                id: user.id,
                body: message.body,
                created_by: user.id,
                created_at: message.created_at,
            },
            ChannelModelResponse {
                id: channels[0].id,
                name: channels[0].name.clone(),
            },
        );

        // output the message to the client as confirmation
        self.send_session_id(
            session_id.clone(),
            Output::UserMessage(response_packet.clone()),
        )
        .await;

        // send to the rest of the clients
        self.send_except_session_id(session_id, Output::Message(response_packet))
            .await;
    }

    async fn process_keep_alive(&self) {
        let alive_interval = self.alive_interval;
        loop {
            println!("pinging all session!");
            time::sleep(alive_interval.unwrap()).await;
            self.send(Output::KeepAliveTick).await;
        }
    }

    /////////////////////
    //  The following seeries of send() functions handle the logic of directing responses to the response_sender stream
    // TODO: if we want to make this horizontally scalable we need a pub/sub solution 
    // to "echo" messeages to/from other services in the cluster.. for this use case probably redis
    /////////////////////
    async fn send(&self, output: Output) {
        println!("!!! Attempting to send !!!");
        if self.response_sender.receiver_count() == 0 {
            println!("Aborting send - no clients listening!");
            return;
        }

        println!("We have a recievers, sending out!");

        let sessions = self.sessions.read().await;
        for (user_id, _session) in sessions.iter() {
            println!("flushing out messages to session: {}", user_id);
            self.response_sender
                .send(ResponsePacket::new(*user_id, *user_id, output.clone()))
                .unwrap();
        }
    }

    async fn send_session_id(&self, session_id: Uuid, output: Output) {
        // println!("!!! send_session_id out messages to session: {}", session_id);
        if self.response_sender.receiver_count() == 0 {
            println!("Aborting send_session_id - no clients listening!");
            return;
        }

        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|session| session.id != session_id)
            .for_each(|session| {
                println!("send_session_id flushing out messages to session: {}", session.id);
                self.response_sender
                    .send(ResponsePacket::new(session.id, session_id, output.clone()))
                    .unwrap();
            });
    }

    // Send a message to everyone but the specified session ID
    async fn send_except_session_id(&self, session_id: Uuid, output: Output) {
        println!(" send_except_session_id out messages to session: {}", session_id);
        if self.response_sender.receiver_count() == 0 {
            println!("Aborting send_session_id - no clients listening!");
            return;
        }

        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|session| session.id == session_id)
            .for_each(|session| {
                println!("send_except_session_id flushing out messages to session: {}", session.id);
                self.response_sender
                    .send(ResponsePacket::new(session.id, session_id, output.clone()))
                    .unwrap();
            });
    }

    async fn send_error(&self, session_id: Uuid, error: ErrorOutput) {
        println!("sending errors!");
        self.send_session_id(session_id, Output::Error(error)).await;
    }
}

impl Default for Holocaster {
    fn default() -> Self {
        Self::new(HolocasterConfig::default())
    }
}
