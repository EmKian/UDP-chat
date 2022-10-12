use std::cell::RefCell;
use std::env::{self, args};
use std::io::{self, stdin, BufRead, BufWriter, stdout, Write};
use std::net::{SocketAddr, ToSocketAddrs, Ipv4Addr, IpAddr};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::RwLock;
use std::task::Poll;
use std::net::UdpSocket;
use std::process::exit;
use std::str::FromStr;
use std::thread;
use std::time::Duration;
use crossterm::cursor::{MoveUp, MoveToPreviousLine, MoveToNextLine};
use crossterm::style::Print;
use crossterm::terminal::size;
use crossterm::{queue, terminal, cursor, execute, Command};

struct SenderReceiver {
    socket: Arc<UdpSocket>,
    sender: Sender,
    receiver: Arc<RwLock<Receiver>>,
}

impl SenderReceiver {
    pub fn new(port: u16) -> std::io::Result<Self> {
        let socket = Arc::new(UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], port)))?);
        let receiver = Arc::new(RwLock::new(Receiver { socket: Arc::clone(&socket)}));
        let sender = Sender { socket: Arc::clone(&socket), destinations: Vec::new() };
        Ok(Self {
            socket,
            sender,
            receiver,
        })
    }
}

struct Receiver {
    socket: Arc<UdpSocket>,
}

impl Receiver {
    pub fn get_datagram(&self) -> (String, SocketAddr) {
        let mut buf = [0; 10000];
        let (_, address) = self.socket.recv_from(&mut buf).unwrap();
        (unsafe {
        String::from_utf8_unchecked(buf.to_vec())
        }, address)
    }
}

struct Sender {
    socket: Arc<UdpSocket>,
    destinations: Vec<SocketAddr>,
}

impl Sender {
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

    pub fn send_to<A: ToSocketAddrs>(&self, message: &[u8], destination: A) {
        self.socket.send_to(message, destination).unwrap();
    }
}


fn parse_port() -> u16 {
    if let Some(ciao) = args().nth(1) {
        ciao.parse().unwrap()
    } else {
        eprintln!("Usage: udp-chat [port]");
        exit(1);
    }
}

fn get_input() -> Result<String, std::io::Error> {
    let mut input = String::new();
    print!("> ");
    io::stdout().flush()?;
    stdin().lock().read_line(&mut input)?;
    Ok(input)
}


fn parse_sockaddr(arguments: &[&str]) -> Option<SocketAddr> {
    let mut arguments = arguments.iter();
    let address = match arguments.next() {
        Some(&"localhost") => "127.0.0.1",
        Some(addr) => addr,
        None => return None,
    };
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


fn draw_history(history: &[String], message: Option<String>) -> Result<(), std::io::Error> {
    let mut stdout = stdout();
    queue!(stdout, cursor::Hide)?;
    queue!(stdout, cursor::SavePosition)?;
    queue!(stdout, cursor::MoveUp(1))?;
    queue!(stdout, terminal::Clear(terminal::ClearType::CurrentLine))?;
    queue!(stdout, terminal::Clear(terminal::ClearType::FromCursorUp))?;
    queue!(stdout, cursor::MoveTo(0, 0))?;
    let message_length = match &message {
        Some(message) => message.lines().count(),
        None => 0, 
    };
    for entry in history.iter().skip(history.len().saturating_sub(usize::from(size()?.1) - message_length - 1)) {
        for lines in entry.lines() {
            queue!(stdout, Print(lines))?;
        }
        queue!(stdout, MoveToNextLine(1))?;
    }
    queue!(stdout, cursor::RestorePosition)?;
    queue!(stdout, MoveUp(1))?;
    for entry in message.unwrap_or_default().lines().rev() {
        queue!(stdout, Print(entry.to_string().trim()))?;
        queue!(stdout, MoveToPreviousLine(1))?;
    }
    queue!(stdout, cursor::RestorePosition)?;
    queue!(stdout, cursor::Show)?;
    stdout.flush()?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    execute!(stdout(), terminal::EnterAlternateScreen)?;
    execute!(stdout(), terminal::Clear(terminal::ClearType::All))?;
    let port = parse_port();
    let mut socket = match SenderReceiver::new(port) {
        Ok(binded) => binded,
        Err(error) => {
            eprintln!("Couldn't bind to port {port}: {error}");
            exit(1);
        }
    };
    let history = Arc::new(RwLock::new(Vec::new()));
    let mut message: Option<String> = None;
    let history_thread = Arc::clone(&history);
    let mut nick = String::new();
    thread::spawn(move || {
        let history = history_thread;
        loop {
            let (datagram, address) = socket.receiver.read().unwrap().get_datagram();
            history.write().unwrap().push(format!("{}\t{}", datagram, address));
            draw_history(&history.read().unwrap(), None).unwrap();
        }
    });
    loop {
        let input = get_input().unwrap();
        match input.chars().next() {
            Some('/') => {
                let arguments = input.chars().skip(1).collect::<String>();
                let mut arguments = arguments.split_whitespace();
                match arguments.next().unwrap_or_default().trim() {
                    "nick" => {
                        nick = arguments.take(1).collect::<String>();
                        socket.sender.send_to_destinations(format!("Changed nick to {}", nick).as_bytes());
                    }
                    "exit" => break,
                    "add" => {
                        match parse_sockaddr(&(arguments.take(2).collect::<Vec<&str>>())) {
                            Some(addr) => socket.sender.add_destination(addr),
                            None => message = Some(String::from("usage: /add [address:port] or [address port]")),
                        }
                    },
                    "whoami" => {
                        message = Some(format!("Your port is: {}", port));
                    }
                    "list" => {
                        let mut string = String::new(); 
                        for address in &socket.sender.destinations {
                            string.push_str(&format!("{address}\n"));
                        }
                        message = Some(string);
                    },
                    _ => (),
                }
            },
            Some(..) => {
                let datagram = if nick.is_empty() {
                    input
                } else {
                    format!("{}: {}", nick, input)
                };
                socket.sender.send_to_destinations(datagram.as_bytes())
            },
            None => (),
        }
        draw_history(&history.read().unwrap(), message.take())?;
    }
    execute!(stdout(), terminal::LeaveAlternateScreen)?;
    Ok(())
}
