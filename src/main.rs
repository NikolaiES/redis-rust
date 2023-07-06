use std::io::{Read, Write};
use std::net::TcpListener;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                println!("accepted new connection");
                let mut buffer = vec![0; 24];
                while let Ok(n) = _stream.read(&mut buffer) {
                    if n == 0 { // connection closed
                        break;
                    }
                    let _ = _stream.write(b"+PONG\r\n");
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
