# bvChat
An implementation of an IRC Server and Client written in Rust!

## Setup/Installation

### Install curl
```
apt-get update
apt-get dist-upgrade -y
apt-get install curl
```
### Install Rust
```
curl -sSf https://static.rust-lang.org/rustup.sh | sh
```

## Use Guide
### Start Server
```
cargo run --bin server
```
### Start Client
```
cargo run --bin client $IP $PORT
```
'localhost' is a valid local IP
Port is 3000 by default

*Note that server must be running to make connection :)
