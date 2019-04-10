use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::error::Error;
use std::fs::File;
use std::fmt;
use std::io;
use std::io::{BufReader, prelude::*};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;

#[derive(Debug)]
struct AuthenticationError;

impl fmt::Display for AuthenticationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Incorrect password entered")
    }
}
impl Error for AuthenticationError {}

struct UserData {
    socket: TcpStream,
    user_id: i32,
}

type UserList = HashMap<String, UserData>;

impl Ord for UserData {
    fn cmp(&self, other: &UserData) -> Ordering {
        self.user_id.cmp(&other.user_id)
    }
}

impl PartialOrd for UserData {
    fn partial_cmp(&self, other: &UserData) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for UserData {
    fn eq(&self, other: &UserData) -> bool {
        self.user_id == other.user_id
    }
}
impl Eq for UserData {}

#[derive(Debug)]
enum Message {
    // TODO: Add any other options for other actions/commands
    // TODO: Decide to use tuple enum or struct enum
    Chat(String,String), // (sender,contents)
    DirectMessage { from: String, to: String, contents: String }, // struct enum
    // DirectMessage(String,String,String), // (from, to, contents) // tuple enum
    Exit(String), // (username)
    Login(String, TcpStream), // (username)
    Motd()
}

fn check_login(username: &str, password: &str) -> bool {
    // FIXME: Parse saved logins file to verify this. Create account if nonexistent
    true
}

/// Check if a user is the admin of the server
fn is_admin(username: &str, user_list: &UserList) -> bool {
    match get_admin(user_list) {
        Some(user) => user == username,
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

/// Build a Message to send a DirectMessage to another user
fn tell(from: &str, contents: &str) -> Message {
    // FIXME: Parse/Pattern match the contents and determine who to send to
    let from = String::from(from);
    let to = String::from("to");
    let contents = String::from("hello");

    Message::DirectMessage { from, to, contents }
}
        
fn handle_connection(
    mut buffer: BufReader<TcpStream>,
    tx: mpsc::Sender<Message>,
    username: String
) -> Result<(), Box<dyn Error>> {
    // Proceed to check for input and send to channel
    loop {
        // TODO: Check if the socket has been closed due to the server
        // We may not have to do this, as the connection being closed
        // should trigger the client to also send this response

        let mut message = String::new();
        buffer.read_line(&mut message);

        // Check if message was 0 bytes, and close the connection if so
        if message.len() == 0 { 
            tx.send(Message::Exit(username));
            return Ok(());
        }
        // TODO: Check if this user is spamming their messages

        // Check for commands
        if message.starts_with("/") {
            // Find which command this is
            let mut index = message.find(" ");
            // If there isn't a space, get the whole word
            let contents = message.split_off(*index.get_or_insert_with( || message.len()));
            match message.as_ref() {
                "/help" => {
                    println!("/who - Diplsays list of all users");
                    println!("/exit - Disconnects from server and quit client");
                    println!("/tell user message - Sends direct message to specified chat");
                    println!("/motd - Diplsays message of the day");
                    println!("/me - Display emote message");
                    println!("/help - Display commands... You did it!");
                    println!("/block user - Prevents user from recieving message from specified user");
                    println!("/unblock user - Allow user to unblock previously blocked user");
                    println!();
                    println!("ADMIN ONLY COMMANDS");
                    println!("/kick user - Kick user from server");
                    println!("/ban user - Immediately kicks user from server and dissallows reconnection");
                    println!("/unban user - Removes ban on specified user");
                }
                "/tell" => { 
                    tx.send(tell(&username, &contents));
                },
                "/motd" => {
                    tx.send(Message::Motd());
                }
                _ => {}
            }
        } else {
            // Broadcast message
            tx.send(Message::Chat(username.clone(), message.clone()));
        }

        println!("{}", message);
    }
    Ok(())
}

macro_rules! broadcast {
    ($list:expr, $($arg:tt)*) => {
        let message = format!($($arg)*);
        let bytes = message.as_bytes();
        $list.iter_mut().for_each(|(_key, val)| {
            val.socket.write(bytes).expect("Failed to send");
        });

    }
}

fn handle_server(rx: mpsc::Receiver<Message>) {
    let mut user_list: UserList = HashMap::new();
    let mut user_id = 0;

    loop {
        match rx.recv().unwrap() {
            Message::Chat(username, message) => {
                //println!("{}: {}", username, message);
                let body = format!("{}: {}", username, message);
                // Send to all sockets in user_list
                broadcast!(user_list, "{}: {}", username, message);
            }
            Message::Login(username, socket) => {
                // TODO: Check if they're already logged in and close the connection. \
                // Try to do this gracefully, as the socket is used by another thread \
                // that may not know we're closing the connection 

                // TODO: Check if they've logged in too many times and disallow it

                // TODO: Check and send any saved DMs

                // Add to hashmap
                broadcast!(user_list, "{} has connected", username); 
                user_list.insert(username, UserData { socket, user_id });
                user_id += 1;
            }
            Message::DirectMessage { from, to, contents } => {
                let text = format!("{} tells you: {}", from, contents);

                match user_list.entry(to) {
                    Occupied(mut d) => {   
                        d.get_mut().socket.write(text.as_bytes());
                    },
                    Vacant(_) => {
                        // TODO: There isn't a user by this name logged in
                        // Save the message for later 
                    }
                }

            }
            Message::Exit(username) => {
                broadcast!(user_list, "{} has exited.", username);
            }
            Message::Motd() => {
                let mut f = File::open("motd.txt").expect("Error opening file.");
                let mut contents = String::new();
                println!("{}",f.read_to_string(&mut contents).expect("Unable to read file."));
            }
            _ => { }
        }
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

    for stream in listener.incoming() {
        println!("User connected to server.");
        let mut stream = stream?;
        let mut socket = stream.try_clone()?;

        // TODO: Check if this user is banned

        let mut username = String::new();
        let mut password = String::new();

        let mut buffer = BufReader::new(stream);

        buffer.read_line(&mut username);
        buffer.read_line(&mut password);

        // Strip newline characters from end
        username.pop();
        password.pop();

        if check_login(&username, &password) {
            println!("Successful login attempt");
            tx.send(Message::Login(username.clone(), socket.try_clone().unwrap()));

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
            println!("Failed login attempt");
            writeln!(socket, "Invalid login credentials");
        }
    }

    Ok(())
}
