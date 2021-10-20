use std::io::prelude::*;
use std::io::{stdin, stdout};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::{self};
use std::time::Duration;

use crate::common::{self, read_messages, send_string, setup_stream};

pub fn join(addr: SocketAddr, username: Option<&str>) {
    let mut stream = Arc::new(RwLock::new(
        TcpStream::connect(&addr).expect("Failed to connect to server."),
    ));
    println!("Connected {}", addr.to_string());

    let username = username.map_or_else(get_username, |u| u.into());

    loop {
        if let Ok(mut stream) = stream.try_write() {
            setup_stream(&stream).expect("Failed to setup connection");

            handshake(&mut stream, &username);

            break;
        }
    }

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();
    ctrlc::set_handler(move || {
        println!("Received Ctrl-C");
        running_clone.store(false, Ordering::SeqCst);
    })
    .expect("Failed to set ctrl-c handler");

    let reader_running_clone = running.clone();
    let stream_clone = stream.clone();
    let reader = thread::Builder::new()
        .name("reader".into())
        .spawn(move || {
            while reader_running_clone.load(Ordering::SeqCst) {
                if let Ok(mut stream) = stream_clone.try_write() {
                    match read_messages(&mut stream) {
                        Ok(Some(msgs)) => {
                            for msg in msgs {
                                println!("{}", msg);
                            }
                        }
                        Err(e) => {
                            eprintln!("{:?}", e);
                        }
                        _ => (),
                    }
                }
                thread::yield_now();
                thread::sleep(Duration::from_millis(10));
            }
        })
        .expect("Could not setup reader");

    chat(&mut stream, &username, &running);

    running.store(false, Ordering::SeqCst);
    reader.join().expect("Failed to wait for reader");

    loop {
        if let Ok(stream) = stream.try_write() {
            stream
                .shutdown(Shutdown::Both)
                .expect("Failed disconnecting...");
            break;
        }
    }
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

pub fn handshake(mut stream: &mut TcpStream, username: &str) {
    // TODO: Handle invalid username errors

    println!("DEBUG: Handshaking...");

    common::send_string(&mut stream, common::SUPER_SECRET_CLIENT_HANDSHAKE.into()).unwrap();

    let welcome = common::read_to_string(&mut stream).expect("Could not read from server");

    if welcome != common::SUPER_SECRET_SERVER_HANDSHAKE {
        panic!("Failed to join server")
    }

    println!("DEBUG: Success");

    common::send_string(&mut stream, username.into()).expect("Failed to write username");
}

pub fn chat(stream: &mut Arc<RwLock<TcpStream>>, username: &str, running: &Arc<AtomicBool>) {
    let pre = format!("{}: ", username);
    let mut msg;

    while running.load(Ordering::SeqCst) {
        msg = readline(&pre);

        match msg.as_str() {
            "/exit" => {
                println!("Exiting...");
                break;
            }
            _ if msg.len() >= 1 => {
                send_msg(stream, &msg);
            }
            _ => continue,
        }
    }
}

pub fn send_msg(stream: &mut Arc<RwLock<TcpStream>>, msg: &str) {
    loop {
        if let Ok(mut stream) = stream.try_write() {
            send_string(&mut stream, format!("{}\n", msg))
                .expect("Failed to send message to server");
            break;
        }
    }
}
