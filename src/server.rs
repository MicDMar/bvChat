use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::fmt;
use std::io::{BufReader, prelude::*};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::thread;
use std::thread::sleep;

#[cfg(test)]
mod tests;

const BAN_FILE: &str = "banned_users.txt";
const COMMAND_TEXT: &str = r#"AVAILABLE COMMANDS:
    /who - Diplsays list of all users
    /exit - Disconnects from server and quit client
    /tell user message - Sends direct message to specified chat
    /motd - Diplsays message of the day
    /me - Display emote message
    /help - Display commands... You did it!
    /block user - Prevents user from recieving message from specified user
    /unblock user - Allow user to unblock previously blocked user

    ADMIN ONLY COMMANDS
    /kick user - Kick user from server
    /ban user - Immediately kicks user from server and dissallows reconnection
    /unban user - Removes ban on specified user"#;

const SPAM_DELAY: u64 = 20;

type UserList = HashMap<String, UserData>;
struct UserData {
    socket: TcpStream,
    user_id: i32,
}

#[derive(Debug)]
enum Message {
    // TODO: Add any other options for other actions/commands
    // TODO: Decide to use tuple enum or struct enum
    Ban(String,String), // (banner,bannee)
    Chat(String,String), // (sender,contents)
    DirectMessage { from: String, to: String, contents: String }, // struct enum
    Exit(String), // (username)
    Kick(String,String), // (kicker,kickee)
    Login(String, TcpStream), // (username)
    Motd(String), // (username)
    Help(String), //(username)
    Spam(String), // (username)
    Unban(String,String), // (banner,bannee)
    Me(String),
    Who(String)
}

fn lines_from_file(filename: impl AsRef<Path>) -> Vec<String> {
    // let file = File::open(filename).expect("No such path file exists.");
    match OpenOptions::new().read(true).open("userdata.txt") {
        Ok(file) => {
            let buffer = BufReader::new(file);
            buffer.lines().map( |l| l.expect("Error parsing.")).collect()
        }
        _ => {
            vec![]
        }
    }
}

fn check_login(username: &str, password: &str) -> bool {
    let lines = lines_from_file("userdata.txt");
    let mut user_hash = HashMap::new();
    let mut f = OpenOptions::new().write(true).create(true).append(true).open("userdata.txt").unwrap();
    
    let mut i = 0;
    while i < lines.len() {
        user_hash.insert(lines[i].to_string(), lines[i+1].to_string());
        i += 2;
    }
    
    if user_hash.contains_key(&username.to_owned()) {
        if user_hash.get(&username.to_owned()).expect("Could not find hash") == password { true }
        else { false }
    }
    else {
        if let Err(e) = writeln!(f, "{}", username) {
            eprintln!("Couldn't write to file: {}", e);
        }
        if let Err(e) = writeln!(f, "{}", password) {
            eprintln!("Couldn't write to file: {}", e);
        }
        true
    }
}

/// Check if a user is the admin of the server
fn is_admin(username: &str, user_list: &mut UserList) -> bool {
    match get_admin(user_list) {
        Some(user) => {
            if user == username {
                true
            } else {
                match user_list.entry(String::from(username)) {
                    Occupied(mut d) => {   
                        let mut socket = &d.get_mut().socket;
                        writeln!(socket, "You are not the admin");
                    },
                    _ => { }
                }
                false
            }
        }
        None => false
    }
}

/// Find the user with the lowest user_id
fn get_admin(user_list: &UserList) -> Option<String> {
    match user_list.iter().min_by(|a, b| a.1.user_id.cmp(&b.1.user_id)) {
        Some(admin_username) => Some(admin_username.0.to_string()),
        None => None
    }
}

fn check_ban(username: &str) -> bool {
    match OpenOptions::new().read(true).open(BAN_FILE) {
        Ok(file) => {
            let buf = BufReader::new(file);
            for line in buf.lines() {
                let line = line.unwrap();
                if line == username {
                    return true;
                }
            }
            false
        }
        Err(_) => false
    }
}

fn ban(username: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(BAN_FILE) {
        if let Err(e) = writeln!(file, "{}", username) {
            eprintln!("Couldn't ban user: {}", username);
        }
    }
}

/// Build a Message to send a DirectMessage to another user
fn tell(from: &str, to: &str, contents: &str) -> Message {
    let from = String::from(from);
    let to = String::from(to);
    let contents = String::from(contents);

    Message::DirectMessage { from, to, contents }
}

struct TimeoutCounter {
    attempts: VecDeque<Instant>,
    max_attempts: usize,
    clear_time: Option<Instant>, // The time the counter was triggered
    penalty_delay: u64, // How long until we reset the count after triggering
    window_size: u64,
}

impl TimeoutCounter {
    fn new(max_attempts: usize, penalty_delay: u64, window_size: u64) -> Self {
        let attempts = VecDeque::with_capacity(max_attempts);

        TimeoutCounter {
            attempts,
            max_attempts,
            clear_time: None,
            penalty_delay,
            window_size,
        }
    }

    fn mark(&mut self) {
        if self.attempts.len() == self.max_attempts {
            self.attempts.pop_front();
        }

        self.attempts.push_back(Instant::now());
    }

    fn triggered(&mut self) -> bool {
        // Check if we've previously triggered this
        match self.clear_time {
            Some(instant) => {
                if instant.elapsed() > Duration::new(self.penalty_delay, 0) {
                    // Clear the stored times
                    self.attempts.clear();
                    self.clear_time = None;
                    return false;
                } else {
                    return true;
                }
            }
            None => { }
        }

        if self.attempts.len() < self.max_attempts {
            false
        } else {
            let now = Instant::now();
            match self.attempts.front() {
                Some(instant) => {
                    let triggered = instant.elapsed() < Duration::new(self.window_size, 0);
                    if triggered {
                        self.clear_time = Some(Instant::now()); 
                    }
                    triggered
                }
                None => false
            }
        }
    }
}

fn handle_connection(
    mut buffer: BufReader<TcpStream>,
    tx: mpsc::Sender<Message>,
    username: String
) -> Result<(), Box<dyn Error>> {
    // Proceed to check for input and send to channel
    let mut timeout = TimeoutCounter::new(5, SPAM_DELAY, 5);

    loop {
        let mut message = String::new();
        buffer.read_line(&mut message);

        // Check if message was 0 bytes, and close the connection if so
        if message.len() == 0 { 
            tx.send(Message::Exit(username));
            return Ok(());
        }

        // Check if this user is spamming their messages
        timeout.mark();
        if timeout.triggered() {
            tx.send(Message::Spam(username.clone()));
        } else {

            // Check for commands
            if message.starts_with("/") {
                // Find which command this is
                let mut index = message.find(" ");
                // If there isn't a space, get the whole word
                let contents = message.split_off(*index.get_or_insert_with(|| message.len() - 1));
                match message.as_ref() {
                    "/help" => {
                        tx.send(Message::Help(username.clone()));
                    }
                    "/tell" => { 
                        let contents = String::from(&contents[1..contents.len()-1]); // Extract rest of string
                        match contents.find(' ') {
                            Some(index) => {
                                tx.send(tell(&username, &contents[0..index], &contents[index+1..contents.len()]));
                            }
                            None => {
                                tx.send(tell("Server", &username, "Invalid /tell format"));
                            }
                        }
                    },
                    "/motd" => {
                        tx.send(Message::Motd(username.clone()));
                    }
                    "/exit" => {
                        tx.send(Message::Exit(username.clone()));
                        break;
                    }
                    "/ban" => {
                        let contents = String::from(&contents[1..contents.len()-1]);
                        tx.send(Message::Ban(username.clone(), contents.clone())); 
                        tx.send(Message::Kick(username.clone(), contents));
                    }
                    "/unban" => {
                        let contents = String::from(&contents[1..contents.len()-1]);
                        tx.send(Message::Unban(username.clone(), contents));
                    }
                    "/kick" => {
                        let contents = String::from(&contents[1..contents.len()-1]);
                        tx.send(Message::Kick(username.clone(), contents));
                    }
                    "/me" => {
                        tx.send(Message::Me(username.clone()));
                    }
                    "/who" => {
                        tx.send(Message::Who(username.clone()));
                    }
                    _ => {}
                }
            } else {
                // Broadcast message
                tx.send(Message::Chat(username.clone(), message.clone()));
            }
        }

        println!("{}", message);
    }
    Ok(())
}

macro_rules! broadcast {
    ($list:expr, $($arg:tt)*) => {
        let message = format!($($arg)*);
        $list.iter_mut().for_each(|(_key, val)| {
            writeln!(val.socket, "{}", message);
        });
    }
}

fn handle_server(rx: mpsc::Receiver<Message>) {
    let mut user_list: UserList = HashMap::new();
    let mut user_id = 0;

    let mut saved_messages: HashMap<String,Vec<Message>> = HashMap::new();

    loop {
        match rx.recv().unwrap() {
            Message::Chat(username, message) => {
                //let body = format!("{}: {}", username, message);
                // Send to all sockets in user_list
                broadcast!(user_list, "{}: {}", username, message);
            }
            Message::Me(username) => {
                broadcast!(user_list, "{} says hi", username);
            }
            Message::Login(username, mut socket) => {
                // Check if they're already logged in and close the connection
                if user_list.contains_key(&username) {
                    writeln!(socket, "This name is already in use");
                    socket.shutdown(Shutdown::Both);
                }
                // Add to hashmap
                broadcast!(user_list, "{} has connected", username); 
                user_list.insert(username.clone(), UserData { socket, user_id });
                user_id += 1;

                // Send all saved messages to the user
                match saved_messages.entry(username.clone()) {
                    Occupied(mut d) => {   
                        d.get_mut().drain(..).for_each(|message| {
                            if let Message::DirectMessage { from, to, contents } = message {
                                match user_list.entry(to.clone()) {
                                    Occupied(mut d) => {   
                                        let mut socket = &d.get_mut().socket;
                                        writeln!(socket, "{} tells you: {}", from, contents);
                                    }
                                    _ => { }
                                }
                    
                            }
                        });
                    }
                    _ => { }
                }

            }
            Message::DirectMessage { from, to, contents } => {
                match user_list.entry(to.clone()) {
                    Occupied(mut d) => {   
                        let mut socket = &d.get_mut().socket;
                        writeln!(socket, "{} tells you: {}", from, contents);
                    },
                    Vacant(_) => {
                        // There isn't a user by this name logged in
                        // Save the message for later 
                        match saved_messages.entry(to.clone()) {
                            Occupied(mut d) => {
                                d.get_mut().push(Message::DirectMessage { from, to, contents});
                            }
                            Vacant(o) => {
                                o.insert(vec![Message::DirectMessage { from, to, contents }]);
                            }
                        }
                    }
                }

            }
            Message::Exit(username) => {
                match user_list.entry(username.clone()) {
                    Occupied(mut d) => {   
                        d.get_mut().socket.shutdown(Shutdown::Both);
                        d.remove_entry();
                    },
                    _ => { }
                }
                broadcast!(user_list, "{} has exited.", username);
            }
            Message::Motd(username) => {
                let mut f = File::open("motd.txt").expect("Error opening file.");
                let mut contents = String::new();
                f.read_to_string(&mut contents).expect("Unable to read file.");
                let text = format!("{}", contents);
                match user_list.entry(username) {
                    Occupied(mut d) => {   
                        d.get_mut().socket.write(text.as_bytes());
                    },
                    _ => { }
                }
            }
            Message::Help(username) => {
                match user_list.entry(username) {
                    Occupied(mut d) => {   
                        let mut socket = &d.get_mut().socket;
                        writeln!(socket, "{}", COMMAND_TEXT);
                    },
                    _ => {
                    }
                }
            }
            Message::Spam(username) => {
                match user_list.entry(username) {
                    Occupied(mut d) => {   
                        let mut socket = &d.get_mut().socket;
                        writeln!(socket, "Too many messages sent too quickly. Please wait 20 seconds before sending again.");
                    },
                    _ => { }
                }

            }
            Message::Ban(banner, bannee) => {
                if is_admin(&banner, &mut user_list) {
                    ban(&bannee);
                    broadcast!(user_list, "{} has been banned.", bannee);
                }
            }
            Message::Unban(banner, bannee) => {
                if is_admin(&banner, &mut user_list) {
                    unban(&bannee);
                    broadcast!(user_list, "{} has been unbanned.", bannee);
                }
                
            }
            Message::Who(username) => {
                let mut user_vec = Vec::new();
                for (user, _) in &user_list {
                    user_vec.push(user.clone())
                }
                //let mut user_vec: Vec<String> = user_list.iter().map(|(user, _)| user).collect::<Vec<String>>();
                match user_list.entry(username) {
                    Occupied(mut d) => {
                        let mut socket = &d.get_mut().socket;
                        for user in user_vec {
                            writeln!(socket, "{}", user);
                        }
                    }
                    Vacant(_) => {}
                }
            }
            Message::Kick(kicker, kickee) => {
                if is_admin(&kicker, &mut user_list) {
                    match user_list.entry(kickee) {
                        Occupied(mut d) => {   
                            let socket = &d.get_mut().socket;
                            socket.shutdown(Shutdown::Both);
                            d.remove_entry();
                        },
                        _ => { }
                    }
                }

            }
            _ => { }
        }
    }
}

fn unban(username: &str) {
    // Search and remove the line for this user if it exists
    if let Ok(file) = File::open(BAN_FILE) {
        let buf = BufReader::new(file); 
        let new_contents: Vec<String> = buf.lines().filter_map(|line| {
            match line { 
                Ok(line) => {
                    if line == username {
                        return None
                    }
                    Some(line)
                }
                Err(_) => None
            }
        }).collect();
        let mut file = OpenOptions::new().write(true).truncate(true).open(BAN_FILE).unwrap();
        new_contents.iter().for_each(|line| { file.write(line.as_bytes()); });
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // TODO: Read the address and port from some kind of input
    let address = "0.0.0.0";
    let port = 3000;

    let listener = TcpListener::bind(format!("{}:{}", address, port))?;

    // mpsc - allow all client threads to send contents to server thread's single buffer
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        handle_server(rx);
    });

    let mut attempts: HashMap<String, TimeoutCounter> = HashMap::new();

    for stream in listener.incoming() {
        let stream = stream?;
        let ip = stream.peer_addr()?.ip().to_string();
        println!("Connection received from {}", ip);
        let mut socket = stream.try_clone()?;

        // Check if this ip has made too many attempts
        match attempts.entry(ip.clone()) {
            Occupied(mut o) => {   
                let counter = o.get_mut();
                if counter.triggered() {
                    writeln!(socket, "Too many connections. Please wait 2 minutes");
                    continue;
                }
            },
            _ => { }
        }


        let mut username = String::new();
        let mut password = String::new();

        let mut buffer = BufReader::new(stream);

        buffer.read_line(&mut username);
        buffer.read_line(&mut password);

        // Strip newline characters from end
        username.pop();
        password.pop();

        if check_ban(&username) {
            writeln!(socket, "You have been banned from this server.");
            continue;
        }

        if check_login(&username, &password) {
            println!("Successful login attempt");
            tx.send(Message::Login(username.clone(), socket.try_clone().unwrap()));
            tx.send(Message::Motd(username.clone()));

            let thread_tx = mpsc::Sender::clone(&tx); // Clone the transmitter so the thread can have its own

            thread::spawn(move || {
                match handle_connection(buffer, thread_tx, username) {
                    Err(err) => {
                        writeln!(socket, "{}", err);
                    }
                    Ok(_) => { }
                }
                println!("User disconnected from server.");
            });
        } else {
            writeln!(socket, "Failed authentication");
            socket.shutdown(Shutdown::Both);
            println!("Failed login attempt");
            match attempts.entry(ip) {
                Occupied(mut o) => {
                    let counter = &mut o.get_mut();
                    counter.mark();
                }
                Vacant(o) => {
                    o.insert(TimeoutCounter::new(3, 120, 30));
                }
            }

        }
    }

    Ok(())
}
