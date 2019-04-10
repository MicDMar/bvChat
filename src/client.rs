use std::env;
use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, Write, prelude::*};
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
    //Leave this here because we need to reopen it to "refresh" the block_list?
    let mut file = OpenOptions::new().read(true).create(true).open("block_list.txt").expect("Failed to open the file.");
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Couldn't read from block_list.txt.");

    let mut message = String::new();
    reader.read_line(&mut message).expect("Unable to read from buffer.");
    
    //TODO check if they don't have colon.
    let mut username: &str = message.split(":").collect()[0];

    //Check the username to see if they are blocked.
    if contents.contains(username){
      //Ignore blocked users message.
      continue;
    }
    else {
      //Print out the message as normal.
      print!("{}", message);
    }

  }
}

fn send_messages(mut stream: TcpStream){
  //Create a new string here, ask for input and send it to the server.
  let mut input = String::new();
  
  loop {
    io::stdin().read_line(&mut input).expect("Failed to read from stdin.");
    
    if input.starts_with("/block"){
      //Add the username that follows to the block list.
      let mut file = OpenOptions::new().append(true).create(true).open("block_list.txt").unwrap();
      let username = input.split_off(7);
      writeln!(file, "{}", username).expect("Failed to write username to file.");
      print!("{} has been successfully blocked!", username);
    }
    else if input.starts_with("/unblock"){
      //Remove the username that follows from the block list.
      let username = input.split_off(9);
      let mut file = OpenOptions::new().read(true).write(true).create(true).open("block_list.txt").unwrap();
      let mut contents = String::new();
      file.read_to_string(&mut contents).expect("Couldn't read from block_list.txt.");
      //Parse through the contents and remove the matching username (if it exists) then write it back to file.
      let mut v: Vec<&str> = contents.split(" ").collect();

      let mut count = 0;
      for name in v{
        let mut check_name: String = String::from(name);
        if check_name == username{
          v.remove(count);
        }
        count += 1;
      }

      let mut new_contents = String::new();
      for name in v{
        let mut new_name: String = String::from(name);
        new_name.push_str(" ");
        new_contents.push_str(&new_name);
      }

      file.write(new_contents.as_bytes());
      print!("{} has been successfully unblocked!", username);
    }
    else { 
      stream.write(input.as_bytes()).expect("Failed to write to the server.");
    }
    
    input.clear();
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
