use super::{UserData, UserList};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream};
use std::thread;

macro_rules! t {
    ($e:expr) => {
        match $e {
            Ok(t) => t,
            Err(e) => panic!("received error for `{}`: {}", stringify!($e), e),
        }
    }
}

#[test]
fn get_admin() {
    let a = Ipv4Addr::new(127, 0, 0, 1);
    let p = 12345;
    let e = SocketAddr::V4(SocketAddrV4::new(a, p));

    let listener = t!(TcpListener::bind(&e));

    thread::spawn(move || {
        let mut list: UserList = HashMap::new();
        list.insert(String::from("test") , UserData { socket: TcpStream::connect(&e).unwrap() , user_id: 0 });
        assert_eq!(super::get_admin(&list), Some(String::from("test")));
    });
}

#[test]
fn is_admin() {
    let a = Ipv4Addr::new(127, 0, 0, 1);
    let p = 12346;
    let e = SocketAddr::V4(SocketAddrV4::new(a, p));

    let listener = t!(TcpListener::bind(&e));

    thread::spawn(move || {
        let mut list: UserList = HashMap::new();
        list.insert(String::from("test") , UserData { socket: TcpStream::connect(&e).unwrap() , user_id: 0 });
        assert!(super::is_admin("test", &list));
    });

}
