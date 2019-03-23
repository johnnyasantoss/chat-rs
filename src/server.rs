use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::string::String;
use std::sync::mpsc::channel;
use std::sync::*;
use std::thread;

use crate::result::Result;

pub struct User {
    login: String,
}

impl User {
    pub fn new(login: String) -> Self {
        User {
            login,
        }
    }
}

pub struct Server {
    users: Vec<User>,
    addr: SocketAddr,
}

impl Server {
    /// Creates a new `Server` instance
    pub fn new(addr: SocketAddr) -> Self {
        Server {
            users: vec![],
            addr,
        }
    }

    pub fn start(self) -> Result<()> {
        println!("Starting server @ {}", self.addr);

        match TcpListener::bind(self.addr) {
            Ok(tcp_listener) => self.readloop(tcp_listener),
            Err(_) => panic!("Failed to bind."),
        }

        Ok(())
    }

    fn readloop(self, tcp_listener: TcpListener) {
        let (greet_sender, greet_recv) = channel();
        let lock = Arc::new(Mutex::new(self));

        thread::Builder::new()
            .name("greeter".into())
            .spawn(move || {
                for username in greet_recv {
                    lock.lock().unwrap().join(User::new(username));
                }
            })
            .unwrap();

        loop {
            for incoming in tcp_listener.incoming() {
                let mut stream = incoming.unwrap();

                let a = greet_sender.clone();
                thread::spawn(move || {
                    println!("Incoming stream {:?}", stream);
                    Server::welcome_new_user(&mut stream);

                    let username = String::new();
                    a.send(username.clone()).unwrap();

                    thread::Builder::new()
                        .name(username.clone())
                        .spawn(move || {
                            Server::serve_chat(username, stream);
                        })
                        .unwrap();
                });
            }
        }
    }

    fn serve_chat(username: String, stream: TcpStream) {}

    fn welcome_new_user(stream: &mut TcpStream) {
        stream.write_all(b"Welcome!").expect("Failed to handshake");
    }

    fn handle_msg(stream: &mut TcpStream) {
        stream.write_all(b"Welcome!").expect("Write");
    }

    pub fn join(&mut self, user: User) -> &mut Self {
        self.announce(&user);

        self.users.push(user);

        self
    }

    pub fn announce(&mut self, user: &User) -> &mut Self {
        println!("New user joined the party! Welcome {}!", user.login);

        self
    }
}
