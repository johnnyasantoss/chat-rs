use result::Result;

use std::io::{Read, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::string::String;
use std::sync::mpsc::channel;
use std::thread;

pub struct User {
    login: String,
    status: String,
}

pub struct Server<'a> {
    users: Vec<&'a User>,
    location: String,
    port: u16,
}

impl<'a> Server<'a> {
    /// Creates a new `Server` instance
    pub fn new(location: &str, port: u16) -> Self {
        return Server {
            users: vec![],
            location: location.to_owned(),
            port: port,
        };
    }

    pub fn start(self) -> Result<()> {
        println!("Starting server @ {}:{}", self.location, self.port);

        let ip_addr = self.location
            .parse::<IpAddr>()
            .expect("Invalid server location");

        let addr = SocketAddr::new(ip_addr, self.port);

        match TcpListener::bind(addr) {
            Ok(tcp_listener) => self.readloop(tcp_listener),
            Err(_) => panic!("Failed to bind."),
        }

        return Ok(());
    }

    fn readloop(&self, tcp_listener: TcpListener) {
        loop {
            for incoming in tcp_listener.incoming() {
                match incoming {
                    Ok(stream) => {
                        let (tx, rx) = channel::<TcpStream>();
                        tx.send(stream).expect("Error: Send");

                        thread::spawn(move || {
                            let mut stream = rx.recv().expect("Error");
                            Server::handle_msg(&mut stream);
                        });
                    }
                    Err(e) => panic!("aaaaaa: {}", e),
                }
            }
        }
    }

    fn handle_msg(stream: &mut TcpStream) {
        stream
            .write(b"HTTP/1.0 200 OK\nContent-Type: text/plain\nContent-Length: 5\n\nPong!\n")
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
