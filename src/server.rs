use std::borrow::BorrowMut;
use std::error::Error;
use std::fmt::Display;
use std::io::{self, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::string::String;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::*;
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub const SUPER_SECRET_CLIENT_HANDSHAKE: &'static str = "Hello!";
pub const SUPER_SECRET_SERVER_HANDSHAKE: &'static str = "Welcome!";

#[derive(Debug)]
pub struct User {
    name: String,
    stream: Box<TcpStream>,
    sender: Sender<Action>,
}

impl User {
    fn new(login: String, stream: Box<TcpStream>, sender: Sender<Action>) -> Self {
        User {
            name: login,
            stream,
            sender,
        }
    }
}

pub struct Server {
    users: Vec<Arc<Mutex<Box<User>>>>,
    addr: SocketAddr,
}

enum Action {
    Greet(String),
    Goodbye(String),
    Broadcast { msg: String, username: String },
    Shutdown,
}

#[derive(Debug)]
pub enum ServerError {
    FailedHandshake,
    UserShutdown,
    Other(Box<dyn Error>),
}
impl Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Server error")
    }
}

impl Error for ServerError {}

impl From<std::io::Error> for ServerError {
    fn from(e: std::io::Error) -> Self {
        ServerError::Other(Box::new(e))
    }
}

impl From<Box<dyn Error>> for ServerError {
    fn from(e: Box<dyn Error>) -> Self {
        ServerError::Other(e)
    }
}

impl From<mpsc::SendError<Action>> for ServerError {
    fn from(e: mpsc::SendError<Action>) -> Self {
        ServerError::Other(Box::new(e))
    }
}

impl Server {
    /// Creates a new `Server` instance
    pub fn new(addr: SocketAddr) -> Self {
        Server {
            users: Vec::new(),
            addr,
        }
    }

    pub fn start(&mut self) -> Result<(), ServerError> {
        println!("Starting server @ {}", self.addr);

        let tcp_listener = TcpListener::bind(self.addr).expect("Failed to bind.");

        self.serve(tcp_listener)?;

        Ok(())
    }

    fn serve(&mut self, listener: TcpListener) -> Result<(), ServerError> {
        let (sender, recv) = channel::<Action>();

        let writter = create_writter(recv)?;

        let mut joins = Vec::<JoinHandle<()>>::new();

        for incoming in listener.incoming() {
            let mut stream = match incoming {
                Ok(s) => s,
                Err(e) => {
                    println!("Error connecting new listener: {:?}", e);
                    continue;
                }
            };

            setup_stream(&stream);

            let name = get_user(&mut stream)?;

            let user = self
                .welcome_new_user(name.clone(), Box::new(stream), sender.clone())
                .expect("Failed to welcome new user");

            let user_ref = self.users.get(user).unwrap().clone();

            let user_thread = thread::Builder::new()
                .name(format!("User: {}", name))
                .spawn(move || {
                    Server::serve_chat(user_ref);
                })?;

            joins.push(user_thread);
        }

        println!("Shuting down...");

        for handle in joins {
            if let Err(e) = handle.join() {
                println!("Error finishing thread {:?}", e);
            }
        }

        sender
            .send(Action::Shutdown)
            .expect("Could not shutdown writter thread");

        writter.join().expect("Failed to shutdown");

        Ok(())
    }

    fn serve_chat(user: Arc<Mutex<Box<User>>>) {
        loop {
            if let Ok(mut user) = user.lock() {
                let stream = user.stream.as_mut();

                if let Ok(Some(e)) = stream.take_error() {
                    println!("{:?}", e);
                    break;
                }

                let msg = match read_to_string(stream) {
                    Ok(m) if m.len() == 0 => continue,
                    Ok(m) => m,
                    Err(e) => {
                        if e.is::<ServerError>() {
                            let err = (*e).downcast_ref::<ServerError>().unwrap();
                            if let ServerError::UserShutdown = err {
                                break;
                            }
                        }

                        let err = (*e).downcast_ref::<io::Error>().unwrap();
                        let kind = err.kind();
                        if kind == ErrorKind::BrokenPipe {
                            break;
                        }
                        if kind == ErrorKind::TimedOut || kind == ErrorKind::Interrupted {
                            continue;
                        }
                        println!("Err: [{:?}] {:?}", kind, &err);
                        continue;
                    }
                };

                if let Err(e) = user.sender.send(Action::Broadcast {
                    msg,
                    username: user.name.clone(),
                }) {
                    println!("Failed to broadcast message: {:?}", e);
                }
            }
        }

        if let Ok(user) = user.lock() {
            user.sender
                .send(Action::Goodbye(user.name.clone()))
                .expect("Failed to gracefully shutdown user thread");
        }
    }

    fn welcome_new_user(
        &mut self,
        username: String,
        stream: Box<TcpStream>,
        sender: Sender<Action>,
    ) -> Result<usize, ServerError> {
        // send the username to the greeter thread
        sender.send(Action::Greet(username.clone()))?;

        let user = Arc::new(Mutex::new(Box::new(User::new(username, stream, sender))));

        let user_pos = self.users.len();
        self.users.push(user);

        Ok(user_pos)
    }
}

fn get_user(stream: &mut TcpStream) -> Result<String, ServerError> {
    let username: String;

    handshake_client(stream)?;

    username = read_to_string(stream)?;

    Ok(username)
}

pub fn setup_stream(stream: &TcpStream) {
    stream.set_nodelay(true).expect("set_nodelay failed");
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("set_read_timeout failed");
    stream
        .set_write_timeout(Some(Duration::from_secs(1)))
        .expect("set_write_timeout failed");
}

fn handshake_client(stream: &mut TcpStream) -> Result<(), ServerError> {
    match read_to_string(stream) {
        Ok(handshake) if handshake == SUPER_SECRET_CLIENT_HANDSHAKE => {
            send_string(stream, SUPER_SECRET_SERVER_HANDSHAKE.to_owned())?;

            Ok(())
        }
        Err(e) => Err(ServerError::from(e)),
        _ => Err(ServerError::FailedHandshake),
    }
}

pub fn send_string(stream: &mut TcpStream, msg: String) -> Result<(), Box<dyn Error>> {
    stream.write_all(msg.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn greet_user(user: &str) {
    println!("New user joined the party! Welcome {}!", user);
    announce(&user);
}

pub fn announce(user: &str) {}

pub fn read_to_string(stream: &mut TcpStream) -> Result<String, Box<dyn Error>> {
    let mut buf = [0u8; 1024];

    //TODO: read bigger messages

    let read: usize;

    loop {
        match stream.read(&mut buf) {
            Ok(n) => {
                if n == 0 {
                    return Err(Box::new(ServerError::UserShutdown));
                }
                read = n;
                break;
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => continue,
            Err(e) => return Err(Box::new(e)),
        }
    }

    let msg = String::from_utf8((buf[..read]).to_vec())
        .expect("Failed to parse string received from client");

    Ok(msg)
}

fn create_writter(receiver: Receiver<Action>) -> Result<JoinHandle<()>, io::Error> {
    let writter = thread::Builder::new()
        .name("writter".into())
        .spawn(move || {
            writter(receiver);
        })?;

    Ok(writter)
}

fn writter(receiver: Receiver<Action>) {
    for action in receiver {
        match action {
            Action::Greet(username) => {
                greet_user(&username);
            }
            Action::Goodbye(username) => {
                println!("Bye bye {}!", username);
            }
            Action::Shutdown => {
                // TODO: Send message to clients to shutdown
                println!("writter: Shutdown");
                break;
            }
            Action::Broadcast { msg, username } => {
                println!("{}: {}", username, msg);
            }
            _ => continue,
        }
    }
}
