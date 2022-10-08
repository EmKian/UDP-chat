use std::env::{self, args};
use std::io::{self, stdin, BufRead};
use std::net::{SocketAddr, ToSocketAddrs, Ipv4Addr, IpAddr};
// use std::net::UdpSocket;
use tokio::net::UdpSocket;
use std::process::exit;
use std::str::FromStr;
use std::thread;
use std::time::Duration;

struct SenderReceiver {
    port: u16,
    socket: UdpSocket,
    destinations: Vec<SocketAddr>,
}

impl SenderReceiver {
    pub async fn new(port: u16) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], port))).await?;
        Ok(Self {
            port,
            socket,
            destinations: Vec::new(),
        })
    }

    pub async fn add_destination<T: ToSocketAddrs>(&mut self, addresses: T) {
        for address in addresses.to_socket_addrs().unwrap() {
            self.destinations.push(address);
        }
    }

    pub async fn send_to_destinations(&self, message: &[u8]) {
        for address in &self.destinations {
            self.socket.send_to(message, address);
        }
    }
}

struct Sender {}

fn parse_port() -> u16 {
    if let Some(ciao) = args().nth(1) {
        ciao.parse().unwrap()
    } else {
        eprintln!("Usage: udp-chat [port]");
        exit(1);
    }
}

fn parse_sockaddr(arguments: &[&str]) -> Option<SocketAddr> {
    let mut arguments = arguments.iter();
    let address = arguments.next()?;
    SocketAddr::from_str(address)
        .ok()
        .or_else(|| {
            let address = IpAddr::from_str(address).ok()?;
            let port: u16 = arguments.next()?.parse().ok()?;
            Some(SocketAddr::new(address, port))
        })
    // match arguments {
    //     [addr] => addr.parse().ok(),
    //     [addr, port, ..] => Some(SocketAddr::new(addr.parse().ok()?, port.parse().ok()?)),
    //     _ => None,
    // }
}


#[tokio::main]
async fn main() -> std::io::Result<()> {
    let port = parse_port();
    let mut socket = match SenderReceiver::new(port).await {
        Ok(binded) => binded,
        Err(error) => {
            eprintln!("Couldn't bind to port {port}: {error}");
            exit(1);
        }
    };
    loop {
        let mut input = String::new();
        stdin().lock().read_line(&mut input).unwrap();
        match input.chars().next() {
            Some('/') => {
                let arguments = input.chars().skip(1).collect::<String>();
                let mut arguments = arguments.split_whitespace();
                match arguments.next().unwrap_or_default().trim() {
                    "exit" => break,
                    "add" => {
                        match parse_sockaddr(&(arguments.take(2).collect::<Vec<&str>>())) {
                            Some(addr) => socket.add_destination(addr),
                            None => eprintln!("usage: /add [address:port] or [address port]"),
                        }
                    },
                    "list" => {
                        for address in &socket.destinations {
                            println!("{address}");
                        }
                    },
                    _ => (),
                }
            },
            Some(..) => socket.send_to_destinations(input.as_bytes()),
            None => (),
        }
        thread::sleep(Duration::from_secs(1));
    }
    Ok(())
}
