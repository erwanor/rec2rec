#![allow(dead_code)]
#![allow(unused_imports)]
use clap::Parser;
use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinError,
};

enum Status {
    Up,
    Down,
}

struct Node {
    status: Status,
    address: Ipv4Addr,
}

#[derive(Parser, Debug)]
struct Args {
    bind_address: String,
}

#[derive(Debug)]
enum Messages {
    Ping,
    Pong,
}

/// This handles incoming connections to our listener
///
/// Currently, it listens for "ping" and says "pong".
async fn work(mut stream: TcpStream, s: SocketAddr) {
    println!("got connection from {s}!");

    println!("server reading string");
    let mut buffer = [0; 10];
    let _characters_recieved = stream
        .read(&mut buffer)
        .await
        .expect("could not read message");

    // TODO: it seems like we never reach this point
    println!("done reading string");

    dbg!(buffer);

    // if buffer != b"ping" {
    //     panic!("did not get ping: {buffer}");
    // }

    stream
        .write_all(b"pong\n")
        .await
        .expect("failed to write to client stream");
}

/// This handles incoming connections to our listener
///
/// Currently, it says "ping".
async fn client_work(mut stream: TcpStream) {
    stream
        .write_all(b"ping\n")
        .await
        .expect("failed to write to stream");

    println!("i sent a ping to {stream:?}");

    // should we wait a bit after sending the ping?
    // better, can we wait until we recieve data?
    let mut buffer = String::new();
    let characters_recieved = stream
        .read_to_string(&mut buffer)
        .await
        .expect("could not read message");

    dbg!(&buffer);
    dbg!(characters_recieved);

    if buffer != "pong" {
        panic!("did not get pong: {buffer}");
    }
}

#[tokio::main]
async fn main() {
    let args: Args = Args::parse();

    let bind_address = args.bind_address;
    let listener = tokio::spawn(async move {
        let listener = TcpListener::bind(&bind_address).await.unwrap();
        println!("started listening on {bind_address}");
        loop {
            let (stream, addr) = listener.accept().await.unwrap();
            tokio::spawn(work(stream, addr));
        }
    });

    // this is a list of peers to connect to
    // in the future, we would like to build this list dynamically
    // for now, we are starting our implementation by enumerating peers
    let peers: Vec<SocketAddrV4> = vec![
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080), // ourself!
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8081),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8082),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8083),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8084),
    ];

    // attempt to connect to all peers
    for peer in peers {
        let stream = TcpStream::connect(&peer);

        match stream.await {
            // this is sending outgoing client connections
            Ok(stream) => {
                tokio::spawn(client_work(stream));
                continue;
            }
            Err(e) => {
                eprintln!("unable to connect to {peer}: {e}");
                continue;
            }
        }
    }

    println!("starting rec2rec!!!");

    // config

    // once we have additional tasks, we would await them here as well
    // we may need to join all of the futures together
    let _ = listener.await;

    // currently our tokio task runs forever
    // when we connect to listeners, we want to also continue listening
    // we need to run both client connections and the listener at the same time
}
