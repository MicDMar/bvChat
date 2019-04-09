use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::str;
use std::thread;

fn login(mut stream: &TcpStream){
  //Ask for user to input their username and their password.
  let mut username = String::new();
  let mut password = String::new();

  print!("Please enter your username: ");
  io::stdout().flush();
  io::stdin().read_line(&mut username).expect("Failed to read from stdin.");
    
  print!("Please enter your password: ");
  io::stdout().flush();
  io::stdin().read_line(&mut password).expect("Failed to read from stdin."); 
  
  stream.write(username.as_bytes()).expect("Failed to write to the server.");
  stream.write(password.as_bytes()).expect("Failed to write to the server.");

}

fn handle_incoming_messages(mut stream: TcpStream){
  //This is basically what we're going to do to read input from the server.
  let mut reader = BufReader::new(stream);
  loop {
    let mut message = String::new();
    //Check the username to see if they are blocked and ignore them.
    reader.read_line(&mut message).expect("Unable to read from buffer.");
    print!("{}", message);
  }
}

fn send_messages(mut stream: TcpStream){
  //Create a new string here, ask for input and send it to the server.
  let mut input = String::new();
  loop {
    io::stdin().read_line(&mut input).expect("Failed to read from stdin.");
    stream.write(input.as_bytes()).expect("Failed to write to the server.");
  }
}

fn main() {
  let mut address = env::args().nth(1).unwrap();
  let mut port = env::args().nth(2).unwrap();
  
  let mut stream = TcpStream::connect(format!("{}:{}", address, port)).expect("Could not connect to server.");
  let mut stream2 = stream.try_clone().expect("Failed to clone this shit.");
  
  login(&stream);
  
  thread::spawn(|| {
    handle_incoming_messages(stream);
  });

  send_messages(stream2);
  
}
