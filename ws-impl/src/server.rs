use std::net::TcpListener;

use crate::holo::handshake;

fn main () {
    let server = TcpListener::bind("127.0.0.1:9000").unwrap();

    for steam in server.incoming() {
        spawn(move || {
            let mut socket = 
        })
    }
}