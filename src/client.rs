use std::io::{Read, stdin, stdout, Write};
use std::net::{SocketAddr, TcpStream};

pub fn join(addr: SocketAddr) {
    match TcpStream::connect(addr) {
        Ok(stream) => {
            println!("Connected {}", addr.to_string());
            let username = get_user_login();

            let mut client = ChatClient::new(username, stream);
            client.handshake();
            client.chat();
        }
        Err(err) => panic!("Failed to connect to server. Error: {}", err),
    }
}

fn readline(pre: &str) -> String {
    print!("{}", pre);
    stdout().flush().unwrap();

    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();

    input
}

fn get_user_login() -> String {
    loop {
        let mut input = readline("Username: ");

        let x: &[_] = &['\n', '\r'];
        input = input.trim_matches(x).to_owned();

        if input.trim().len() != input.len() {
            println!("No trailing spaces allowed.");
            continue;
        }
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
        let mut line_str = String::new();

        self.stream
            .read_to_string(&mut line_str)
            .expect("Error when handshaking");

        println!("Joining...");
        self.stream.write_all(self.username.as_bytes()).unwrap();
    }

    pub fn chat(&mut self) {
        let pre = format!("{}: ", self.username);
        loop {
            let msg = readline(&pre);

            self.send_msg(&msg);
        }
    }

    pub fn send_msg(&mut self, msg: &str) {
        self.stream
            .write_all(msg.as_bytes())
            .expect("Error when sending message");
    }
}
