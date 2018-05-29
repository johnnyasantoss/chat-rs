extern crate clap;

use clap::{App, Arg, SubCommand};

// internals
mod client;
mod server;

fn main() {
    let serverArg = Arg::with_name("server")
        .short("s")
        .help("Server to connect")
        .takes_value(true)
        .default_value("localhost");

    let portArg = Arg::with_name("port")
        .short("p")
        .help("Port to connect")
        .takes_value(true)
        .default_value("1337")
        .validator(|v| match v.parse::<i32>() {
            Ok(_) => return Ok(()),
            Err(_) => Err("Port should be a integer".to_string()),
        });

    let app = App::new("chat")
        .author("Johnny Santos <johnnyadsantos@gmail.com>")
        .about("A chat using tcp. Made for learning purposes")
        .bin_name("chat-rs")
        .version("0.1.0")
        .version_message("Genesis")
        .subcommand(
            SubCommand::with_name("join")
                .about("Join a chat server")
                .arg(&serverArg)
                .arg(&portArg),
        )
        .subcommand(
            SubCommand::with_name("server")
                .about("Start a chat server")
                .arg(&serverArg)
                .arg(&portArg),
        );

    let matches = app.get_matches();

    if let Some(matches) = matches.subcommand_matches("join") {
        if let Some(server) = matches.value_of("server") {
            client::join(server)
        } else {
            client::join(&String::from("localhost"))
        }
    }

    if let Some(matches) = matches.subcommand_matches("server") {
        if let Some(server) = matches.value_of("server") {
            server::start(server)
        }
    }
}
