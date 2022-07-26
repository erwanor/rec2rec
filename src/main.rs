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
use std::sync::Arc;
use tokio::io::BufWriter;
use tokio::sync::mpsc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinError,
};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
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

async fn peer_handler(mut tx: Arc<mpsc::Sender<Command>>, mut peer: Peer) {
    peer.ping().await;
    loop {
        match peer.read().await {
            Ok(Some(Message::Ping)) => {
                // tx.send(Command::MessageReceived(peer.addr, Message::Ping));
                peer.pong().await;
            }
            Ok(Some(Message::Pong)) => {
                info!("HANDSHAKE COMPLETED!")
            }
            Ok(Some(Message::Info(msg))) => {
                info!("peer_handler: received info {msg}");
            }
            Ok(None) => info!("peer_handler for {peer:?} Ok(None)'d"),
            Err(e) => {
                info!("error: {e:?}");
                break;
            }
        }
    }
}

async fn client(mut tx: Arc<mpsc::Sender<Command>>, mut rx: mpsc::Receiver<Command>) {
    loop {
        tokio::select! {
            Some(command) = rx.recv() => {
                match command {
                    Command::AddPeer(peer) => {
                        info!("found new peer: {peer:?}");
                        tokio::spawn(peer_handler(tx.clone(), peer));
                    },
                    Command::MessageReceived(from, msg) => {
                        info!("{from:?} sent {msg:?}");
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
    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let args: Args = Args::parse();

    let local_address = args.bind_address.clone();

    let (tx, rx) = mpsc::channel(1);
    let tx = Arc::new(tx);
    let tx2 = tx.clone();
    tokio::spawn(client(tx.clone(), rx));

    let listener = tokio::spawn(async move {
        let listener = TcpListener::bind(&local_address).await.unwrap();
        info!("started listening on {local_address}");
        loop {
            let (stream, addr) = listener.accept().await.unwrap();
            if let SocketAddr::V4(a) = addr {
                tx.send(Command::AddPeer(Peer::new(a, stream))).await;
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
                info!("connecting to {peer:#}");
                let cmd = Command::AddPeer(Peer::new(peer, stream));
                tx2.send(cmd).await;
                continue;
            }
            Err(e) => {
                info!("unable to connect to {peer}: {e}");
                continue;
            }
        }
    }

    let _ = listener.await;
}
