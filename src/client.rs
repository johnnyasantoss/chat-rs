use std::io::prelude::*;
use std::io::{stdin, stdout};
use std::net::{Shutdown, SocketAddr, TcpStream};

use crate::common::{self, send_string, setup_stream};

pub fn join(addr: SocketAddr, username: Option<&str>) {
    let stream = TcpStream::connect(&addr).expect("Failed to connect to server.");
    println!("Connected {}", addr.to_string());

    setup_stream(&stream).expect("Failed to setup connection");

    let username = username.map_or_else(get_username, |u| u.into());

    let mut client = ChatClient::new(username, stream);
    client.handshake();
    client.chat();

    client
        .stream
        .shutdown(Shutdown::Both)
        .expect("Failed disconnecting...");
}

fn readline(pre: &str) -> String {
    print!("{}", pre);
    stdout().flush().unwrap();

    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();

    let x: &[_] = &['\n', '\r', ' '];
    input.trim_matches(x).into()
}

fn get_username() -> String {
    loop {
        let input = readline("Username: ").to_owned();

        let input_len = input.len();

        if input_len > 15 || input_len < 5 {
            println!("Username needs to have 5-15 chars");
            continue;
        }

        return input;
    }
}

pub struct ChatClient {
    stream: TcpStream,
    username: String,
}

impl ChatClient {
    pub fn new(username: String, stream: TcpStream) -> Self {
        ChatClient { stream, username }
    }

    pub fn handshake(&mut self) {
        // TODO: Handle invalid username errors

        println!("DEBUG: Handshaking...");

        common::send_string(
            &mut self.stream,
            common::SUPER_SECRET_CLIENT_HANDSHAKE.into(),
        )
        .unwrap();

        let welcome = common::read_to_string(&mut self.stream).expect("Could not read from server");

        if welcome != common::SUPER_SECRET_SERVER_HANDSHAKE {
            panic!("Failed to join server")
        }

        println!("DEBUG: Success");

        common::send_string(&mut self.stream, self.username.clone())
            .expect("Failed to write username");
    }

    pub fn chat(&mut self) {
        let pre = format!("{}: ", self.username);
        let mut msg;

        loop {
            msg = readline(&pre);

            match msg.as_str() {
                "/exit" => {
                    println!("Exiting...");
                    break;
                }
                _ if msg.len() >= 1 => {
                    self.send_msg(&msg);
                }
                _ => continue,
            }
        }
    }

    pub fn send_msg(&mut self, msg: &str) {
        send_string(&mut self.stream, format!("{}\n", msg))
            .expect("Failed to send message to server")
    }
}
