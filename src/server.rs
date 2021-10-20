use std::io::{self, ErrorKind};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::string::String;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::*;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::common::{
    self, read_messages, read_to_string, send_string, setup_stream, Action, ServerError,
};

#[derive(Debug)]
pub struct User {
    name: String,
    stream: Box<TcpStream>,
}

impl User {
    fn new(login: String, stream: Box<TcpStream>) -> Self {
        User {
            name: login,
            stream,
        }
    }
}

pub fn start(addr: SocketAddr) -> Result<(), ServerError> {
    println!("Starting server @ {}", addr);

    let tcp_listener = TcpListener::bind(addr).expect("Failed to bind.");

    serve(tcp_listener)?;

    Ok(())
}

fn serve(listener: TcpListener) -> Result<(), ServerError> {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    ctrlc::set_handler(move || {
        running_clone.store(false, Ordering::SeqCst);
    })
    .expect("Failed to set ctrl-c handler");

    let mut users = Arc::new(RwLock::new(Vec::<User>::new()));

    let (action_sender, action_receiver) = channel::<Action>();

    let buttler_running = running.clone();
    let buttler_sender = action_sender.clone();
    let buttler =
        create_buttler(listener, buttler_sender, buttler_running).expect("Initialize buttler");

    let writter_sender = action_sender.clone();
    let writter = create_action_processor(action_receiver, writter_sender, users.clone())?;

    let mut serve_sender = action_sender.clone();

    let mut err = None;

    while running.load(Ordering::SeqCst) {
        serve_chat(&mut users, &mut serve_sender).unwrap_or_else(|e| err = Some(e))
    }

    println!("Shutting down main...");

    action_sender
        .send(Action::Shutdown)
        .expect("Could not shutdown writter thread");

    buttler.join().expect("Failed to shutdown");
    writter.join().expect("Failed to shutdown");

    Ok(())
}

fn serve_chat(
    users: &mut Arc<RwLock<Vec<User>>>,
    sender: &mut Sender<Action>,
) -> Result<(), ServerError> {
    if let Ok(mut write_lock) = users.try_write() {
        for user in write_lock.iter_mut() {
            let stream = user.stream.as_mut();

            match read_messages(stream) {
                Ok(None) => continue,
                Ok(Some(messages)) => {
                    for message in messages {
                        sender
                            .send(Action::Broadcast {
                                message,
                                username: user.name.clone(),
                            })
                            .expect(&format!("Failed to broadcast message"));
                    }
                }
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
                        sender
                            .send(Action::Goodbye(user.name.clone()))
                            .expect("Failed to gracefully shutdown user thread");
                        break;
                    }

                    if kind == ErrorKind::TimedOut
                        || kind == ErrorKind::Interrupted
                        || kind == ErrorKind::WouldBlock
                    {
                        continue;
                    }

                    println!("Err: [{:?}] {:?}", kind, &err);
                    continue;
                }
            }
        }
    }

    thread::sleep(Duration::from_micros(5));
    thread::yield_now();

    Ok(())
}

fn get_user(stream: &mut TcpStream) -> Result<String, ServerError> {
    let username: String;

    handshake_client(stream)?;

    username = read_to_string(stream)?;

    Ok(username)
}

fn handshake_client(stream: &mut TcpStream) -> Result<(), ServerError> {
    match read_to_string(stream) {
        Ok(handshake) if handshake == common::SUPER_SECRET_CLIENT_HANDSHAKE => {
            send_string(stream, common::SUPER_SECRET_SERVER_HANDSHAKE.to_owned())?;

            Ok(())
        }
        Err(e) => Err(ServerError::from(e)),
        _ => Err(ServerError::FailedHandshake),
    }
}

fn greet_user(user: &str) {
    println!("New user joined the party! Welcome {}!", user);
}

fn create_action_processor(
    receiver: Receiver<Action>,
    sender: Sender<Action>,
    users: Arc<RwLock<Vec<User>>>,
) -> Result<JoinHandle<()>, io::Error> {
    let writter = thread::Builder::new()
        .name("action_processor".into())
        .spawn(move || {
            writter_loop(receiver, sender, users);
        })?;

    Ok(writter)
}

fn create_buttler(
    listener: TcpListener,
    buttler_sender: Sender<Action>,
    buttler_running: Arc<AtomicBool>,
) -> Result<JoinHandle<()>, io::Error> {
    let buttler = thread::Builder::new()
        .name("buttler".into())
        .spawn(move || {
            listener
                .set_nonblocking(true)
                .expect("Cannot set non-blocking");

            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => receive_new_connection(stream, buttler_sender.clone())
                        .unwrap_or_else(|e| {
                            eprintln!("ERROR: {:?}", e);
                        }),
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        thread::yield_now();
                    }
                    Err(e) => println!("Error connecting new listener: {:?}", e),
                }

                if !buttler_running.load(Ordering::SeqCst) {
                    // Signal has been received. Exiting
                    println!("Buttler shutting down...");
                    return;
                }
            }
        })?;

    Ok(buttler)
}

fn writter_loop(receiver: Receiver<Action>, sender: Sender<Action>, users: Arc<RwLock<Vec<User>>>) {
    for action in receiver {
        match action {
            Action::Goodbye(name) => {
                users.delete_user(&name).unwrap();
                println!("Bye bye {}!", name);
            }
            Action::Dropped(name) => {
                users.delete_user(&name).unwrap();
                println!("INFO: Disconnecting dropped user: {}!", name);
            }
            Action::Shutdown => {
                // TODO: Send message to clients to shutdown
                println!("writter: Shutdown");
                break;
            }
            Action::Broadcast {
                username,
                message: msg,
            } => {
                users.for_each_mut(|user| {
                    if user.name == username {
                        println!("{}: {}", &user.name, &msg);
                    } else {
                        send_string(&mut user.stream, format!("{}: {}", &username, &msg))
                            .unwrap_or_else(|e| {
                                eprintln!("ERROR: Failed broadcasting to {}: {:?}", &user.name, e);
                                sender
                                    .send(Action::Dropped(username.clone()))
                                    .expect("Failed to drop user");
                            });
                    }
                });
            }
            Action::NewUser { username, stream } => {
                greet_user(&username);
                users.add_user(User::new(username, Box::new(stream)));
            }
        }
    }
}
fn receive_new_connection(
    mut stream: TcpStream,
    sender: Sender<Action>,
) -> Result<(), ServerError> {
    setup_stream(&stream).expect("Failed to setup connection");

    let name = get_user(&mut stream)?;

    let mut action = Action::NewUser {
        username: name.clone(),
        stream,
    };

    loop {
        match sender.send(action) {
            Ok(_) => break,
            Err(e) => {
                println!("Failed to add new user: {}", name);
                eprintln!("Error: {}", e);
                action = e.0
            }
        }
    }

    Ok(())
}

trait ManageUsers {
    // fn find_by_username(&self, name: &str) -> Option<&User>;

    fn delete_user(&self, name: &str) -> Result<User, &str>;

    fn add_user(&self, user: User);

    fn for_each_mut<T>(&self, f: T)
    where
        T: FnMut(&mut User);
}

impl ManageUsers for Arc<RwLock<Vec<User>>> {
    // fn find_by_username(&self, name: &str) -> Option<&User> {
    //     loop {
    //         if let Ok(users) = self.try_read() {
    //             return users.iter().find(|u| &u.name == name);
    //         }
    //     }
    // }

    fn delete_user(&self, name: &str) -> Result<User, &str> {
        loop {
            if let Ok(mut users) = self.try_write() {
                if let Some((i, _)) = users.iter().enumerate().find(|(_, u)| &u.name == name) {
                    return Ok(users.remove(i));
                } else {
                    return Err(&"Could not find user");
                }
            }
        }
    }

    fn add_user(&self, user: User) {
        loop {
            if let Ok(mut users) = self.try_write() {
                users.push(user);
                return;
            }
        }
    }

    fn for_each_mut<T>(&self, mut f: T)
    where
        T: FnMut(&mut User),
    {
        loop {
            if let Ok(mut users) = self.try_write() {
                for user in users.iter_mut() {
                    f(user);
                }
                break;
            }
        }
    }
}
