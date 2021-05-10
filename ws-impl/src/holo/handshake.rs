/**
 * Written by Bruce Bjorklund 4/4/2021
 *
 *
 */

use bytes::Buf;
use crate::{
    error::{Error, ProtocolError, SocketResult, CapacityError},
    utils::NonBlockingResult,
};

use http::{
    response::Builder, HeaderMap, Request as HttpRequest, Response as HttpResponse, StatusCode,
};
use input_buffer::{InputBuffer, MIN_READ};
use log::*;
use std::{
    io::{Cursor, Read, Write},
    marker::PhantomData,
    result::Result as StdResult,
};


//////////////////////////////////// 


pub type Request = HttpRequest<()>;
pub type Response = HttpResponse<()>;
pub type ErrorResponse = HttpResponse<Option<String>>;

// The possible states of a handshake round
#[derive(Debug)]
pub enum HandshakeRoundResult<Obj, Stream> {
    // Using WouldBlock because it is an IO error state that we need to be aware of
    WouldBlock(HandshakeStateMachine<Stream>),
    Incomplete(HandshakeStateMachine<Stream>),
    Completed(HandshakeStageResult<Obj, Stream>),
}

// The possible states of a handshake stage
#[derive(Debug)]
pub enum HandshakeStageResult<Obj, Stream> {
    CompleteRead {
        result: Obj,
        stream: Stream,
        tail: Vec<u8>,
    },
    DoneWrite(Stream),
}

// WebSocket Handshake state is either reading or writing
#[derive(Debug)]
enum HandshakeState {
    // Read State to the peer
    Reading(InputBuffer),
    // Write State to the peer
    Writing(Cursor<Vec<u8>>),
}

// Try to parse the stream of bits
pub trait TryParse: Sized {
    fn try_parse(data: &[u8]) -> SocketResult<Option<(usize, Self)>>;
}

// Websocket handshake state machine
#[derive(Debug)]
pub struct HandshakeStateMachine<Stream> {
    stream: Stream,
    state: HandshakeState,
}

impl<Stream> HandshakeStateMachine<Stream> {
    // Handle read-event during handshake, from the stream and update state
    pub fn init_read(stream: Stream) -> Self {
        HandshakeStateMachine {
            stream,
            state: HandshakeState::Reading(InputBuffer::with_capacity(MIN_READ)),
        }
    }

    // Handle the write-event during handshake, from the stream, and then update the state
    pub fn init_write<D: Into<Vec<u8>>>(stream: Stream, data: D) -> Self {
        HandshakeStateMachine {
            stream,
            state: HandshakeState::Writing(Cursor::new(data.into())),
        }
    }
}

impl<Stream: Read + Write> HandshakeStateMachine<Stream> {
    // Kickoff state machine for a handshake-round
    pub fn try_handshake_round<Obj: TryParse>(
        mut self,
    ) -> SocketResult<HandshakeRoundResult<Obj, Stream>> {
        trace!("Starting handshake round!");

        // Switch over the current state of the handshaking process
        match self.state {
            HandshakeState::Reading(mut buf) => {
                let read = buf
                    .prepare_reserve(MIN_READ)
                    .with_limit(usize::max_value())
                    .map_err(|_| Error::Capacity(CapacityError::HeaderTooLong))?
                    .read_from(&mut self.stream)
                    .no_block()?;

                match read {
                    Some(0) => Err(Error::Protocol(ProtocolError::BadHanshake)),
                    Some(_) => Ok(if let Some((size, obj)) = Obj::try_parse(Buf::chunk(&buf))? {
                        buf.advance(size);
                        HandshakeRoundResult::Completed(HandshakeStageResult::CompleteRead {
                            result: obj,
                            stream: self.stream,
                            tail: buf.into_vec(),
                        })
                    } else {
                        HandshakeRoundResult::Incomplete(HandshakeStateMachine {
                            state: HandshakeState::Reading(buf),
                            ..self
                        })
                    }),
                    None => Ok(HandshakeRoundResult::WouldBlock(HandshakeStateMachine {
                        state: HandshakeState::Reading(buf),
                        ..self
                    })),
                }
            }

            // We are Writing to the stream, so
            HandshakeState::Writing(mut buf) => {
                assert!(buf.has_remaining());
                if let Some(size) = self.stream.write(Buf::chunk(&buf))? {
                    assert!(size > 0);
                    buf.advance(size);
                }
            }
        }
    }
}

/////////////////////////////////////////////////////////////////////
/// Handshake wrapper utilizes the statemachine
// Handle Server
// TODO: Add config option
#[derive(Debug)]
pub struct ServerHandy<S, C> {
    cb: Option(C),
    error_response: Option<ErrorResponse>,
    _marker: PhantomData<S>,
}
