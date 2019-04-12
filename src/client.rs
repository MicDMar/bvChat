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

    'outer: loop {
        let mut message = String::new();
        reader.read_line(&mut message).expect("Unable to read from buffer.");

        // TODO: Check if they don't have colon.
        let mut split: Vec<&str> = message.split(":").collect();
        let mut username = String::from(split[0]);
        // username.pop();

        //Leave this here because we need to reopen it to "refresh" the block_list?
        match OpenOptions::new().read(true).open("block_list.txt") {
            Ok(file) => {
                let buf = BufReader::new(file);
                for line in buf.lines() {
                    let mut line = line.unwrap();
                    //Check the username to see if they are blocked.
                    if line == username {
                        // Ignore blocked users message.
                        continue 'outer;
                    } 
                }
            }
            Err(_) => { }
        }
        //Print out the message as normal.
        print!("{}", message);
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
            let mut username = input.split_off(7);
            username.pop();
            
            writeln!(file, "{}", username).expect("Failed to write username to file.");
            println!("{} has been successfully blocked!", username);
        }
        else if input.starts_with("/unblock"){
            //Remove the username that follows from the block list.
            let mut username = input.split_off(9);
            username.pop();

            use std::fs::File;
            if let Ok(file) = File::open("block_list.txt") {
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
                let mut file = OpenOptions::new().write(true).truncate(true).open("block_list.txt").unwrap();
                new_contents.iter().for_each(|line| { file.write(line.as_bytes()); });
            }
            println!("{} has been successfully unblocked!", username);
        }
        else if input.starts_with("/exit"){
          println!("Goodbye :)");
          break;
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
