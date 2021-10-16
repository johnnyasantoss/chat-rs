use std::error::Error;
use std::fmt::Display;
use std::io::{self, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::string::String;
use std::sync::*;
use std::time::Duration;

pub const SUPER_SECRET_CLIENT_HANDSHAKE: &'static str = "Hello!";
pub const SUPER_SECRET_SERVER_HANDSHAKE: &'static str = "Welcome!";

pub fn setup_stream(stream: &TcpStream) -> io::Result<()> {
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(Duration::from_millis(1)))?;
    stream.set_write_timeout(Some(Duration::from_secs(1)))?;

    Ok(())
}

pub fn send_string(stream: &mut TcpStream, msg: String) -> Result<(), Box<dyn Error>> {
    stream.write_all(msg.as_bytes())?;
    stream.flush()?;
    Ok(())
}

pub fn read_to_string(stream: &mut TcpStream) -> Result<String, Box<dyn Error>> {
    let mut buf = [0u8; 1024];

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

pub enum Action {
    Goodbye(String),
    Dropped(String),
    Broadcast { username: String, message: String },
    Shutdown,
    NewUser { username: String, stream: TcpStream },
}

pub fn read_messages(stream: &mut TcpStream) -> Result<Option<Vec<String>>, Box<dyn Error>> {
    if let Ok(Some(e)) = stream.take_error() {
        return Err(Box::new(e));
    }

    let mut buf = [0; 5];
    let has_data = match stream.peek(&mut buf) {
        Ok(read) if read > 0 => true,
        Err(e) => {
            let kind = e.kind();

            if kind == ErrorKind::TimedOut
                || kind == ErrorKind::Interrupted
                || kind == ErrorKind::WouldBlock
            {
                return Ok(None);
            }

            return Err(Box::new(e));
        }
        _ => return Ok(None),
    };

    if !has_data {
        return Ok(None);
    }

    let data = read_to_string(stream)?;

    let messages = data
        .split('\n')
        .filter(|s| s.len() > 0)
        .map(|s| s.to_string())
        .collect();

    Ok(Some(messages))
}