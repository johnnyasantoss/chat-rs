use std::io::{Read, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::string::String;
use std::sync::mpsc::channel;
use std::thread;

use crate::result::Result;

pub struct User {
    login: String,
    status: String,
}

pub struct Server<'a> {
    users: Vec<&'a User>,
    addr: SocketAddr,
}

impl<'a> Server<'a> {
    /// Creates a new `Server` instance
    pub fn new(addr: SocketAddr) -> Self {
        return Server {
            users: vec![],
            addr,
        };
    }

    pub fn start(self) -> Result<()> {
        println!("Starting server @ {}", self.addr);

        match TcpListener::bind(self.addr) {
            Ok(tcp_listener) => self.readloop(tcp_listener),
            Err(_) => panic!("Failed to bind."),
        }

        return Ok(());
    }

    fn readloop(&self, tcp_listener: TcpListener) {
        loop {
            for incoming in tcp_listener.incoming() {
                match incoming {
                    Ok(_stream) => {
                        let (tx, rx) = channel::<TcpStream>();

                        let mut stream = rx.recv().expect("Error");
                        Server::handle_msg(&mut stream);

                        tx.send(stream).expect("Error: Send");
                    }
                    Err(e) => panic!("aaaaaa: {}", e),
                }
            }
        }
    }

    fn handle_msg(stream: &mut TcpStream) {
        stream
            .write(b"Pong!\n")
            .expect("Write");
        println!("Incoming stream {:?}", stream);

        stream.shutdown(Shutdown::Both).expect("Shutdown");
    }

    pub fn join(mut self, user: &'a User) -> Self {
        self.users.push(user);

        return self;
    }

    pub fn announce(self, user: &'a User) -> Self {
        println!("New user joined the party! Welcome {}!", user.login);

        return self;
    }
}
