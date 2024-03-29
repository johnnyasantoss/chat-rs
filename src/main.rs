extern crate clap;

use std::net::{IpAddr, SocketAddr};
use std::u16;

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

// internals
mod client;
mod common;
mod server;

fn main() {
    let server_arg = Arg::with_name("server")
        .short("s")
        .help("Server to connect")
        .takes_value(true)
        .default_value("127.0.0.1");

    let port_arg = Arg::with_name("port")
        .short("p")
        .help("Port to connect")
        .takes_value(true)
        .default_value("1337")
        .validator(|v| match v.parse::<u16>() {
            Ok(_) => Ok(()),
            Err(_) => Err(format!(
                "Port should be a positive value between {} and {}.",
                u16::MIN,
                u16::MAX
            )),
        });

    let username_arg = Arg::with_name("username")
        .long("username")
        .short("u")
        .help("Sets your username")
        .takes_value(true);

    let app = App::new("chat-rs")
        .author("Johnny Santos <johnnyadsantos@gmail.com>")
        .about("A chat using tcp. Made for learning purposes")
        .bin_name("chat-rs")
        .version("0.1.0")
        .subcommand(
            SubCommand::with_name("join")
                .about("Join a chat server")
                .arg(&server_arg)
                .arg(&port_arg)
                .arg(&username_arg),
        )
        .subcommand(
            SubCommand::with_name("server")
                .about("Start a chat server")
                .arg(&server_arg)
                .arg(&port_arg),
        )
        .setting(AppSettings::ColorAuto)
        .setting(AppSettings::SubcommandRequiredElseHelp);

    let matches = app.get_matches();

    if let Some(matches) = matches.subcommand_matches("join") {
        let addr = get_server_addr(&matches);
        let username = matches.value_of("username");

        client::join(addr, username);
    }

    if let Some(matches) = matches.subcommand_matches("server") {
        let addr = get_server_addr(&matches);

        server::start(addr).expect("Failed to serve");
    }
}

fn get_server_addr(matches: &ArgMatches) -> SocketAddr {
    let server = matches.value_of("server").expect("Server address");
    let port = matches
        .value_of("port")
        .expect("Server port")
        .parse::<u16>()
        .expect("Server port isn't a valid u16");

    let ip_addr = server.parse::<IpAddr>().expect("Invalid server location");

    SocketAddr::new(ip_addr, port)
}
