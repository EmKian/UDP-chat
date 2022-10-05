use std::env::{self, args};
use std::io::{self, stdin, BufRead};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::process::exit;
use std::thread;
use std::time::Duration;

struct SenderReceiver {
    port: u16,
    socket: UdpSocket,
    destinations: Vec<SocketAddr>,
}

impl SenderReceiver {
    pub fn new(port: u16) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], port)))?;
        Ok(Self {
            port,
            socket,
            destinations: Vec::new(),
        })
    }

    pub fn add_destination<T: ToSocketAddrs>(&mut self, addresses: T) {
        for address in addresses.to_socket_addrs().unwrap() {
            self.destinations.push(address);
        }
    }

    pub fn send_to_destinations(&self, message: &[u8]) {
        for address in &self.destinations {
            self.socket.send_to(message, address).unwrap();
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

fn main() -> std::io::Result<()> {
    let port = parse_port();
    let mut socket = match SenderReceiver::new(port) {
        Ok(binded) => binded,
        Err(error) => {
            eprintln!("Couldn't bind to port {port}: {error}");
            exit(1);
        }
    };
    socket.add_destination("127.0.0.1:8800".to_string());
    socket.add_destination("127.0.0.1:8801".to_string());
    loop {
        let mut input = String::new();
        stdin().lock().read_line(&mut input).unwrap();
        match input.chars().next() {
            Some('/') => {
                let string: String = input.chars().skip(1).collect();
                match string.trim() {
                    "exit" => break,
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
