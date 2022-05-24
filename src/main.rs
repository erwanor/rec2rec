#![allow(dead_code)]
#![allow(unused_imports)]
use std::net::{Ipv4Addr, SocketAddr};
use tokio::{net::TcpListener, task::JoinError};

enum Status {
    Up,
    Down,
}

struct Node {
    status: Status,
    address: Ipv4Addr,
}

async fn work(s: SocketAddr) {
    println!("doing work! with {}", s);
}

#[tokio::main]
async fn main() -> Result<(), JoinError> {
    println!("starting rec2rec!");
    // config
    tokio::spawn(async move {
        let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
        println!("started listening to lo:8080");
        loop {
            let (_, addr) = listener.accept().await.unwrap();
            tokio::spawn(work(addr));
        }
    })
    .await
}
