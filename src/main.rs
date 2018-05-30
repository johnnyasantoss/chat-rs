extern crate clap;

use clap::{App, AppSettings, Arg, SubCommand};

// internals
mod client;
mod server;

fn get_app() -> App<'static, 'static> {
    let server_arg = Arg::with_name("server")
        .short("s")
        .help("Server to connect")
        .takes_value(true)
        .default_value("localhost");

    let port_arg = Arg::with_name("port")
        .short("p")
        .help("Port to connect")
        .takes_value(true)
        .default_value("1337")
        .validator(|v| match v.parse::<i32>() {
            Ok(_) => return Ok(()),
            Err(_) => Err("Port should be a integer".to_string()),
        });

    let app = App::new("chat-rs")
        .author("Johnny Santos <johnnyadsantos@gmail.com>")
        .about("A chat using tcp. Made for learning purposes")
        .bin_name("chat-rs")
        .version("0.1.0")
        .subcommand(
            SubCommand::with_name("join")
                .about("Join a chat server")
                .arg(&server_arg)
                .arg(&port_arg),
        )
        .subcommand(
            SubCommand::with_name("server")
                .about("Start a chat server")
                .arg(&server_arg)
                .arg(&port_arg),
        )
        .setting(AppSettings::ColorAuto)
        .setting(AppSettings::SubcommandRequiredElseHelp);

    return app;
}

fn main() {
    let app = get_app();
    let matches = app.get_matches();

    if let Some(matches) = matches.subcommand_matches("join") {
        let server = matches.value_of("server").unwrap();
        let port = matches.value_of("port").unwrap().parse::<i32>().unwrap();
        client::join(&server, &port);
    }

    if let Some(matches) = matches.subcommand_matches("server") {
        let server = matches.value_of("server").unwrap();
        let port = matches.value_of("port").unwrap().parse::<i32>().unwrap();
        server::start(&server, &port);
    }
}
