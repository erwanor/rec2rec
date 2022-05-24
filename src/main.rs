#![allow(dead_code)]
#![allow(unused_imports)]
use clap::Parser;
use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::{
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
#[clap()]
struct Args {
    bind_address: String,
}

async fn work(s: SocketAddr) {
    println!("doing work! with {}", s);
}

#[tokio::main]
async fn main() -> Result<(), JoinError> {
    let args: Args = Args::parse();

    // this is a list of peers to connect to
    // in the future, we would like to build this list dynamically
    // for now, we are starting our implementation by enumerating peers
    let peers: Vec<SocketAddrV4> = vec![
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8081),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8082),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8083),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8084),
    ];

    // attempt to connect to all peers
    for peer in peers {
        let stream = TcpStream::connect(&peer);

        match stream.await {
            Ok(_stream) => continue,
            Err(e) => {
                eprintln!("unable to connect to {peer}: {e}");
                continue;
            }
        }
    }

    println!("starting rec2rec!!!");
    // config
    tokio::spawn(async move {
        let listener = TcpListener::bind(args.bind_address).await.unwrap();
        println!("started listening to lo:8080");
        loop {
            let (_, addr) = listener.accept().await.unwrap();
            tokio::spawn(work(addr));
        }
    })
    .await

    // currently our tokio task runs forever
    // when we connect to listeners, we want to also continue listening
    // we need to run both client connections and the listener at the same time
}
