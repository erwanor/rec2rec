#![allow(dead_code)]
#![allow(unused_imports)]
use bytes::{Buf, BufMut, BytesMut};
use clap::Parser;
use core::fmt;
use std::fmt::{Debug, Formatter};
use std::io::{self, Cursor};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs};
use std::str::FromStr;
use tokio::io::BufWriter;
use tokio::sync::mpsc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinError,
};

#[derive(Debug)]
enum State {
    Offline,
    Syncing,
    Connected,
}

struct Peer {
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

struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    fn new(s: TcpStream) -> Self {
        Self {
            stream: BufWriter::new(s),
            buffer: BytesMut::with_capacity(1024),
        }
    }

    async fn write_message(&mut self, m: Message) -> io::Result<()> {
        match m {
            Message::Ping => {
                // self.stream.write_u8(b'>').await?;
                self.stream.write_all("PING".as_bytes()).await?;
            }
            Message::Pong => {
                // self.stream.write_u8(b'>').await?;
                self.stream.write_all("PONG".as_bytes()).await?;
            }
        }
        self.stream.write_all(b"\r\n").await?;
        Ok(())
    }

    pub async fn read_message(&mut self) -> Option<Message> {
        loop {
            if let Some(message) = self.parse_message().await {
                return Some(message);
            }

            let num_bytes = self.stream.read_buf(&mut self.buffer).await.unwrap();
            if num_bytes == 0 {
                if self.buffer.is_empty() {
                    return None;
                } else {
                    // connection reset by peer
                    return None;
                }
            }
        }
    }

    async fn parse_message(&mut self) -> Option<Message> {
        let mut buf = Cursor::new(&self.buffer[..]);
        Message::check(&mut buf)
    }
}

struct Node {
    peers: Vec<Peer>,
    address: Ipv4Addr,
}

#[derive(Parser, Debug)]
struct Args {
    bind_address: String,
}

#[derive(Debug)]
enum Message {
    Ping,
    Pong,
}

#[derive(Debug)]
enum Error {
    Incomplete,
}

fn read_line<'a>(src: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], Error> {
    let start = src.position() as usize;
    let end = src.get_ref().len() - 1;
    for i in start..end {
        if src.get_ref()[i] == b'\r' && src.get_ref()[i + 1] == b'\n' {
            src.set_position((i + 2) as u64);
            return Ok(&src.get_ref()[start..i]);
        }
    }

    Err(Error::Incomplete)
}

impl Message {
    pub fn check(src: &mut Cursor<&[u8]>) -> Option<Message> {
        match src.get_u8() {
            b'>' => {
                let line = read_line(src).unwrap();
                let message = String::from_utf8(line.to_vec()).unwrap();
                match message.as_str() {
                    "PING" => Some(Message::Ping),
                    "PONG" => Some(Message::Pong),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

impl Peer {
    fn new(addr: SocketAddrV4, stream: TcpStream) -> Self {
        Self {
            addr,
            state: State::Offline,
            connection: Connection::new(stream),
        }
    }

    pub async fn ping(&mut self) -> io::Result<()> {
        self.connection.write_message(Message::Ping).await
    }
}

enum Command {
    AddPeer(Peer),
    MessageReceived(SocketAddrV4, Message),
    Quit,
}

async fn client(mut rx: mpsc::Receiver<Command>) {
    let mut peers: Vec<Peer> = Vec::with_capacity(128);
    loop {
        tokio::select! {
            Some(command) = rx.recv() => {
                match command {
                    Command::AddPeer(mut peer) => {
                        println!("found new peer: {peer:?}");
                        tokio::spawn(async move {
                            loop {
                                peer.listen().await;
                            }
                        });
                        peer.ping().await;
                    },
                    Command::MessageReceived(from, msg) => {
                        println!("{from:?} sent {msg:?}");
                    },
                    _ => {},
                }
            }
        }
    }
}

fn peering(p: Peer) -> () {
    unimplemented!()
}

#[tokio::main]
async fn main() {
    let args: Args = Args::parse();

    let local_address = args.bind_address.clone();

    let (tx, rx) = mpsc::channel(1);
    let tx2 = tx.clone();
    tokio::spawn(client(rx));

    let listener = tokio::spawn(async move {
        let listener = TcpListener::bind(&local_address).await.unwrap();
        println!("started listening on {local_address}");
        loop {
            let (stream, addr) = listener.accept().await.unwrap();
            if let SocketAddr::V4(a) = addr {
                tx2.send(Command::AddPeer(Peer::new(a, stream))).await;
            }
        }
    });

    let local_address = args.bind_address.parse::<SocketAddrV4>().unwrap();

    let peers: Vec<SocketAddrV4> = vec![
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8081),
    ];

    for peer in peers {
        if peer == local_address {
            continue;
        }

        let stream = TcpStream::connect(&peer);

        match stream.await {
            Ok(stream) => {
                println!("connecting to {peer:#}");
                let cmd = Command::AddPeer(Peer::new(peer, stream));
                tx.send(cmd).await;
                continue;
            }
            Err(e) => {
                eprintln!("unable to connect to {peer}: {e}");
                continue;
            }
        }
    }

    let _ = listener.await;
}
