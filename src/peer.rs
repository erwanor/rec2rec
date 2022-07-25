use std::fmt::{Debug, Formatter};
use std::fmt;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs};
use tokio::net::TcpStream;
use crate::connection::Connection;
use crate::state::State;
use tokio::io::BufWriter;
use tokio::sync::mpsc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    task::JoinError,
};

use std::io;

use crate::{Message, Error};
use thiserror;

#[derive(Debug, thiserror::Error)]
pub enum PeerError {
    #[error("connection reset by peer")]
    ConnectionError,

}

pub struct Peer {
    addr: SocketAddrV4,
    state: State,
    connection: Connection,
}

impl Debug for Peer {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("peer")
            .field(&self.addr)
            .field(&self.state)
            .finish()
    }
}


impl Drop for Peer {
    fn drop(&mut self) {
        println!("tearing down peer {:?}", self.addr);
        drop(self)
    }
}


impl Peer {
    pub fn new(addr: SocketAddrV4, stream: TcpStream) -> Self {
        Self {
            addr,
            state: State::Offline,
            connection: Connection::new(stream),
        }
    }

    pub async fn ping(&mut self) -> io::Result<()> {
        println!("pinging!");
        self.connection.write_message(Message::Ping).await
    }

    pub async fn pong(&mut self) -> io::Result<()> {
        println!("ponging!");
        self.connection.write_message(Message::Pong).await
    }

    pub async fn read(&mut self) -> Result<Option<Message>, Error> {
        println!("read_message:");
        self.connection.read_message().await
    }
}

