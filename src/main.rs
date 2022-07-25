#![feature(inherent_associated_types)]
#![allow(dead_code)]
#![allow(unused_imports)]
use anyhow::Result;
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
mod connection;
mod message;
mod peer;
mod state;
use crate::message::Message;
use crate::peer::Peer;
use thiserror::Error;

type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Parser, Debug)]
struct Args {
    bind_address: String,
}

enum Command {
    AddPeer(Peer),
    MessageReceived(SocketAddrV4, Message),
    Quit,
}

async fn peer_handler(mut peer: Peer) -> anyhow::Result<()> {
    loop {
        let msg = peer.read().await.unwrap();
        match msg {
            Some(Message::Ping) => {
                peer.pong().await?
            },
            Some(Message::Pong) => {
                println!("HANDSHAKE COMPLETED!")
            },
            Some(_) => todo!(),
            None => {}
        }
    }
}

async fn client(mut rx: mpsc::Receiver<Command>) {
    let mut peers: Vec<Peer> = Vec::with_capacity(128);
    loop {
        tokio::select! {
            Some(command) = rx.recv() => {
                match command {
                    Command::AddPeer(mut peer) => {
                        println!("found new peer: {peer:?}");

                        // Peer handler
                        tokio::spawn(async move {
                            peer.ping().await;
                            loop {
                                println!("handler for peer {:?}...", peer);
                                match peer.read().await {
                                    Ok(Some(msg)) => match msg {
                                        Message::Ping => {
                                            peer.pong().await;
                                        },
                                        Message::Pong => {
                                            println!("HANDSHAKE COMPLETED!");
                                        },
                                        Message::Info(m) => {
                                            println!("received info message: {m}");
                                        }
                                    },
                                    Ok(None) => {},
                                    Err(e) => {
                                        println!("e: {e:?}");
                                        println!("Tearing down peer handler");
                                        break
                                    }
                                }
                            }
                        });
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
