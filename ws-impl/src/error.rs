use http::Response;
/**
 * This page is for handling all error-handling
 */
use std::{io, result, str, string};
use thiserror::Error;

pub type SocketResult<T> = result::Result<T, Error>;

// Generic Errors
#[derive(Error, Debug)]
pub enum Error {
    #[error("HTTP error: {}", .0.status())]
    Http(Response<Option<String>>),

    // TODO: handle all the capcity errors that could occur
    #[error("Space limit exceeded: {0}")]
    Capacity(#[from] CapacityError),

    // IO errors are fatal as fuuuck... Need this or reading from a buffer will be bad joo-joo
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("WebSocket protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    /// HTTP format error.
    #[error("HTTP format error: {0}")]
    HttpFormat(#[from] http::Error),

}
// Implement "From" handling so we can convert error types into our fancy error

// impl From<http::header::InvalidHeaderValue> for Error {
//     fn from(err: http::header::InvalidHeaderValue) -> Self {
//         Error::HttpFormat(err.into())
//     }
// }



// Specific protocol-errors
#[derive(Error, Debug, PartialEq, Eq, Clone, Copy)]
pub enum ProtocolError {
    #[error("Handshake did not complete successfully.")]
    BadHanshake,

}

#[derive(Error, Debug, PartialEq, Eq, Clone, Copy)]
pub enum CapacityError {
    /// Too many headers provided (see [`httparse::Error::TooManyHeaders`]).
    #[error("Too many headers")]
    TooManyHeaders,
    /// Received header is too long.
    #[error("Header too long")]
    HeaderTooLong,
    /// Message is bigger than the maximum allowed size.
    #[error("Message too long: {size} > {max_size}")]
    MessageTooLong {
        /// The size of the message.
        size: usize,
        /// The maximum allowed message size.
        max_size: usize,
    },
    /// TCP buffer is full.
    #[error("Incoming TCP buffer is full")]
    TcpBufferFull,
}