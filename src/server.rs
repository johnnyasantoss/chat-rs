use result::Result;

use std::thread::sleep_ms;

pub struct User {
    login: String,
    status: String,
}

pub struct Server<'a> {
    users: Vec<&'a User>,
    location: String,
    port: i32,
}

impl<'a> Server<'a> {
    /// Creates a new `Server` instance
    pub fn new(location: &str, port: i32) -> Self {
        return Server {
            users: vec![],
            location: location.to_owned(),
            port: port,
        };
    }

    pub fn start(self) -> Result<()> {
        println!("Starting server @ {}:{}", self.location, self.port);

        loop {
            println!("Running!!");
            sleep_ms(250);
        }

        return Ok(());
    }

    pub fn join(mut self, user: &'a User) -> Self {
        self.users.push(user);

        return self;
    }

    pub fn announce(self, user: &'a User) -> Self {
        println!("New user joined the party! Welcome {}!", user.login);

        return self;
    }
}
