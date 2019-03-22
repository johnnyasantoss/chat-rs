//TODO: Impl

use std::io::Write;
use std::net::{SocketAddr, TcpStream};

pub fn join(addr: SocketAddr) {
    match TcpStream::connect(addr) {
        Ok(mut stream) => {
            println!("Joined {}", addr.to_string());

            stream.write(b"Hello!");
        }
        Err(err) => {
            panic!("Failed to connect to server. Error: {}", err)
        }
    }
}
