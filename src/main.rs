#![allow(dead_code)]
#![allow(unused_imports)]
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
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
    // this is a list of peers to connect to
    // in the future, we would like to build this list dynamically
    // for now, we are starting our implementation by enumerating peers
    let _peers: Vec<SocketAddrV4> = vec![
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8081),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8082),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8083),
        SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8084),
    ];

    println!("starting rec2rec!!!");
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
