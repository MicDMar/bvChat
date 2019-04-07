use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
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
}

fn check_login(username: &str, password: &str) -> bool {
    // TODO: Check if this user is banned

    // TODO: Check if the user has the correct password,
    // creating the account if it does not exist
    
    false 
}

fn handle_connection(mut stream: &TcpStream, tx: mpsc::Sender<Message>) -> Result<(), impl Error> {
    let mut username = String::new();
    let mut password = String::new();
    let socket = stream.try_clone().unwrap();
    let mut buffer = BufReader::new(stream);

    buffer.read_line(&mut username);
    buffer.read_line(&mut password);

    if check_login(&username, &password) {
        println!("Successful login attempt");
        tx.send(Message::Login(username.clone(), socket));
        // login => { tx.send(Message::Login(login)); }
    } else {
        println!("Failed login attempt");
        return Err(AuthenticationError {});
    }

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
            // TODO: Command processing
            // use String.split_off (?) to get the rest of the string
        } else {
            // Broadcast message
            tx.send(Message::Chat(username.clone(), message.clone()));
        }

        println!("{}", message);
    }
    Ok(())
}

fn handle_server(rx: mpsc::Receiver<Message>) {
    let mut user_list: HashMap<String, UserData> = HashMap::new();
    let mut user_id = 0;

    match rx.recv().unwrap() {
        Message::Chat(username, message) => {
            println!("{}: {}", username, message);
            // TODO: Format the message's text

            // TODO: Send to all sockets in user_list (iteration)
        }
        Message::Login(username, socket) => {
            // TODO: Check if they're already logged in and close the connection. \
            // Try to do this gracefully, as the socket is used by another thread \
            // that may not know we're closing the connection 

            // TODO: Check if they've logged in too many times and disallow it
               
            // Add to hashmap
            user_list.insert(username, UserData { socket, user_id });
            user_id += 1;
        }
        Message::DirectMessage { from, to, contents } => {
            let text = format!("{} tells you: {}", from, contents);
        }
        Message::Exit(username) => {
            println!("{} has exited.", username);
        }
        _ => { }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // TODO: Read the address and port from some kind of input
    let address = "127.0.0.1";
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
        let thread_tx = mpsc::Sender::clone(&tx); // Clone the transmitter so the thread can have its own

        thread::spawn(move || {
            // TODO: Consider checking login information here, so we can
            // quickly exit and send the username of who exited through tx
            // This may also assist with keeping of times to prevent
            // login after failing authentication too many times

            match handle_connection(&stream, thread_tx) {
                Err(err) => {
                    writeln!(stream, "{}", err);
                }
                Ok(_) => { }
            }
            println!("User disconnected from server.");
        });
    }

    Ok(())
}
