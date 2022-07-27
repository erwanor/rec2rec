use futures::Future;
use tracing::info;
use std::fmt::{Debug, Formatter};
use std::fmt;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs};
use std::pin::Pin;
use std::task::{Poll, Context};
use tokio::net::TcpStream;
use crate::connection::Connection;
use crate::state::State;
use tokio::io::BufWriter;
use tower::Service;
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
    clock: u32,
}

impl Service<Message> for Peer {
    type Response = Message;
    type Error = PeerError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Message) -> Self::Future {
        unimplemented!()
    }
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
        info!("tearing down peer {:?}", self.addr);
        drop(self)
    }
}


impl Peer {
    pub fn new(addr: SocketAddrV4, stream: TcpStream) -> Self {
        Self {
            addr,
            state: State::Offline,
            connection: Connection::new(stream),
            clock: 0,
        }
    }

    pub async fn heartbeat(&mut self) -> io::Result<()> {
        info!("HEARTBEAT({}) to {}", self.clock, self.addr);
        let clock = self.clock;
        self.clock += 1;
        self.connection.write_message(Message::Heartbeat(clock)).await
    }

    pub async fn ping(&mut self) -> io::Result<()> {
        info!("PING to {}", self.addr);
        self.connection.write_message(Message::Ping).await
    }

    pub async fn pong(&mut self) -> io::Result<()> {
        info!("PONG to {}", self.addr);
        self.connection.write_message(Message::Pong).await
    }

    pub async fn read(&mut self) -> Result<Option<Message>, Error> {
        self.connection.read_message().await
    }
}
