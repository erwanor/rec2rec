use crate::message;
use crate::message::Message;
use crate::message::MessageCodec;
use crate::peer;
use crate::Error;
use bytes::{Buf, BufMut, BytesMut};
use std::io::{self, Cursor};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs};
use std::str::FromStr;
use thiserror;
use tokio::io::BufWriter;
use tokio::sync::mpsc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinError,
};
use tokio_util::codec::Decoder;
use tokio_util::codec::Framed;

use futures::SinkExt;
use futures::StreamExt;

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("connection reset by peer!")]
    ResetConnection,
}

pub struct Connection {
    framed: Framed<BufWriter<TcpStream>, message::MessageCodec>,
}

impl Connection {
    pub fn new(s: TcpStream) -> Self {
        Self {
            framed: Framed::new(BufWriter::new(s), MessageCodec {}),
        }
    }

    pub async fn write_message(&mut self, m: Message) -> io::Result<()> {
        self.framed.send(m).await
    }

    pub async fn read_message(&mut self) -> Result<Option<Message>, Error> {
        loop {
            match self.framed.next().await {
                Some(result) => match result {
                    Ok(m) => {
                        return Ok(Some(m));
                    }
                    Err(e) => {
                        return Err(e.into())
                    }
                },
                None => {
                    // get rid of this..
                    return Err(message::ParseError::BadIO.into());
                }
            }
        }
    }
}
