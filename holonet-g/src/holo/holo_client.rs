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
    // a client can belong to multiple channels at once!
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
        let session_id = self.id;

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
                    // Using the session ID so I can test the code flow
                    Ok(RequestPacket::new(session_id, session_id, body))
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
        let session_id = self.id;
        println!("Attempting to write output!!!");

        // 
        while let Some(result) = reciever.recv().await.ok() {
            if result.session_id == session_id {
                println!("got the message to send {}", result.session_id);
                let data = serde_json::to_string(&result.output).unwrap();
                let msg = warp::ws::Message::text(data);
                stream.send(Ok(msg)).unwrap();
            }
        }

        return;
    }
}
