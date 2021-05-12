use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use futures::{StreamExt, TryStreamExt};
// use log::{error, info};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;
// use tokio_stream::{StreamExt};

use warp::ws::WebSocket;
use warp::Filter;

use crate::holo::holo_api::RequestPacket;
use crate::holo::holo_client::HoloClient;
// use crate::holo::holo_errors::{HoloError, Result};
use crate::holo::holocaster::{Holocaster, HolocasterConfig};

const MAX_FRAME_SIZE: usize = 1 * 65535;

pub struct Server {
  port: u16,
  holocaster: Arc<Holocaster>,
}

impl Server {
  pub fn new(port: u16) -> Self {
    Server {
      port,
      holocaster: Arc::new(Holocaster::new(HolocasterConfig {
        alive_interval: Some(Duration::from_secs(20)),
      })),
    }
  }

  // Boot the server
  pub async fn run(&self) {
    println!("HOLONET BOOT SEQUENCE BEGIN");
    println!("MAX FRAME SIZE: {:?}", MAX_FRAME_SIZE);

    // This has shared ownership with Holoc`aster since it is an Arc<T>
    // Meaning that
    let holocaster = self.holocaster.clone();
    let (input_sender, input_receiver) = mpsc::unbounded_channel::<RequestPacket>();
    let shutdown_handler = async {
      tokio::signal::ctrl_c()
        .await
        .expect("Failed to exit service. View ctrl+c signal handler implemementation.");
    };

    // Construct routes, and init the server
    let routes = Self::build_routes(holocaster, input_sender);
    let (_, server) = warp::serve(routes)
      .bind_with_graceful_shutdown(([127, 0, 0, 1], self.port), shutdown_handler);

    // Initialize the Holonet (notice we used the original one, and not the cloend copy above that we passed into the route-handler)
    let holonet = self.holocaster.run(input_receiver);

    tokio::select! {
      _ = server => {
        println!("HTTP server online!")
      },
      _ = holonet => {
        println!("Websocket service online!")},
    }
  }

  // Construct Routes for the application
  fn build_routes(
    holocaster: Arc<Holocaster>,
    input_sender: UnboundedSender<RequestPacket>,
  ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    println!("Booting routes!!");
    let health = warp::path("healh").map(|| "Fear is the path to the dark side. Fear leads to anger; anger leads to hate; hate leads to suffering. I sense much fear in you.");

    // Where all incoming/outgoing messages are piped through
    let socket = warp::path("socket")
      // prepares the websocket handshake
      .and(warp::ws())
      // Make the input-stream and shared-holocaster Warp-Filters...
      .and(warp::any().map(move || input_sender.clone()))
      .and(warp::any().map(move || holocaster.clone()))
      .map(
        move |ws: warp::ws::Ws,
              input_sender: UnboundedSender<RequestPacket>,
              holocaster: Arc<Holocaster>| {
          ws.on_upgrade(move |web_socket| async move {
            println!("Attempting to connect!");
            tokio::spawn(Self::establish_connection(
              holocaster,
              web_socket,
              input_sender,
            ));
          })
        },
      );

    let routes = warp::get().and(health.or(socket));

    return routes;
  }

  async fn establish_connection(
    holocaster: Arc<Holocaster>,
    web_socket: WebSocket,
    input_sender: UnboundedSender<RequestPacket>,
  ) {
    println!("Establishing a new connection!");
    // Generate  a new client
    let default_channels: Vec<Uuid> = Vec::new();
    let holocaster_listener = holocaster.subscribe();

    // Socket is  split into a reciever/sender of messages
    let (ws_sink, ws_stream) = web_socket.split();
    let client = HoloClient::new(default_channels);

    // info!("HoloClient {} connected!", client.id);

    // Generate an unbound channel to  handle buffering and flushing of the socket to the Holocaster
    let (tx, rx) = mpsc::unbounded_channel();
    let rx = UnboundedReceiverStream::new(rx);
    tokio::spawn(rx.forward(ws_sink));

    // HANDLE INPUT STREAM
    // reading will return back a stream 
    let reading = client
      .handle_incoming(ws_stream)
      .try_for_each(|request_packet| async {
        println!("Attempting to read message packet!!");
        input_sender.send(request_packet).unwrap();
        Ok(())
      });

    if let Err(err) = tokio::select! {
        result = reading => result,
        // HANDLE OUTPUT STREAM
        // 
        _message = client
        .write_output(holocaster_listener, tx) => {
          println!("ALL MESSAGES SENT CLOSING OUT!");
          Ok(())
        },
    } {
      println!("HoloClient connection error: {}", err);
      println!("Shutting down connection!");
      println!("HoloClient {} disconnected", client.id);
    }

    holocaster.handle_disconnect(client.id).await;
    println!("!!!! The HoloClient {} DISCONNECTED with client ID:: ", client.id);
  }
}
