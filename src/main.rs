use std::io::{Read, Write, ErrorKind};
use std::net::{TcpListener, TcpStream};

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");
    let mut clients: Vec<TcpStream> = vec![];

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();
    listener.set_nonblocking(true).expect("Cannot set nonblocking");

    loop {
        match listener.accept(){
            Ok( _stream) => {
                println!("accepted new connection");
                _stream.0.set_nonblocking(true).expect("Cannot set nonblocking for socket.");
                clients.push(_stream.0)
            }
            Err(e) => {
                if e.kind() != ErrorKind::WouldBlock {
                    println!("error: {}", e);
                }
            }
        }

        let mut disconnected: Vec<usize> = vec![];
        for (i, client) in clients.iter_mut().enumerate(){
            let mut buffer = [0; 1024];
            match client.read(&mut buffer) {
                Ok(_) => {
                    let _ = client.write(b"+PONG\r\n");
                }
                Err(e) => {
                    if e.kind() == ErrorKind::WouldBlock {
                        continue;
                    } else {
                        println!("error: {}", e);
                        disconnected.push(i);
                    }
                }
            }
        }

        for &index in disconnected.iter().rev() {
            clients.remove(index);
        }

    }

}
