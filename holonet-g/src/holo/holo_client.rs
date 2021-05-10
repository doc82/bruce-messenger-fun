// use std::{error, result};

use futures::stream::SplitStream;
use futures::Stream;
use tokio_stream::StreamExt;
// use tokio::time;
use uuid::Uuid;
// use warp::filters::ws::WebSocket;

use crate::holo::holo_api::{Input, RequestPacket, ResponsePacket};
use crate::holo::holo_errors::{Error, Result};

#[derive(Clone, Default)]
pub struct HoloClient {
    // This is the session UUID
    pub id: Uuid,
    // A session is bound to a channel (it can be changed at any time)
    pub channels: Vec<Uuid>,
}

impl HoloClient {
    // TODO: we can add all sorts of cool stuff in here
    pub fn new(channels: Vec<Uuid>) -> Self {
        // TODO: we should store the session header JWT token here as well for quick lookup downstream?
        HoloClient {
            id: Uuid::new_v4(),
            channels,
        }
    }

    // TODO: make this channel specific
    pub fn handle_incoming(
        &self,
        stream: SplitStream<warp::ws::WebSocket>,
    ) -> impl Stream<Item = Result<RequestPacket>> {
        let client_id = self.id;

        println!("Attempting to handle na incoming connect!!");

        stream
            .take_while(|message| {
                if let Ok(message) = message {
                    message.is_text()
                } else {
                    println!("!! nope validation failed, aborting !!");
                    false
                }
            })
            .map(move |message| match message {
                Err(err) => {
                    println!("!! had a really big error !!");
                    Err(Error::System(err.to_string()))
                }
                Ok(message) => {
                    println!("!! message was good! {}", message.to_str().unwrap());
                    let body: Input = serde_json::from_str(message.to_str().unwrap())?;
                    println!("!! Sending response back to client !!");
                    // TODO: the second param should be a channel id
                    Ok(RequestPacket::new(client_id, client_id, body))
                }
            })
    }

    pub async fn write_output(
        &self,
        mut reciever: tokio::sync::broadcast::Receiver<ResponsePacket>,
        stream: tokio::sync::mpsc::UnboundedSender<
            std::result::Result<warp::ws::Message, warp::Error>,
        >,
    ) {
        println!("Attempting to write output!!!");
        let client_id = self.id;

        while let Some(result) = reciever.recv().await.ok() {
            if result.client_id == client_id {
                println!("got the message to send {}", result.client_id);
                let data = serde_json::to_string(&result.output).unwrap();
                let msg = warp::ws::Message::text(data);
                stream.send(Ok(msg)).unwrap();
            }
        }

        return;
    }
}
