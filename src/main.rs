use anyhow::Result;
use bytes::BytesMut;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

type SharedState = Arc<Mutex<HashMap<String, String>>>;

fn parse_args(command: &str) -> Vec<&str> {
    let lines: Vec<&str> = command.split("\r\n").collect();
    let mut commands_and_args: Vec<&str> = vec![];

    let mut i = 0;
    while i < lines.len() {
        if lines[i].starts_with('*') {
            i += 1;
            continue;
        } else if lines[i].starts_with('$') {
            i += 1;
            if i < lines.len() {
                commands_and_args.push(lines[i]);
            }
        }
        i += 1;
    }
    return commands_and_args;
}

async fn handle_client(mut client: TcpStream, state: SharedState) -> Result<()> {
    let mut buffer = BytesMut::with_capacity(4096);

    loop {
        buffer.clear();
        let bytes_read = client.read_buf(&mut buffer).await?;

        if bytes_read == 0 {
            eprintln!("Client disconnected");
            return Ok(());
        }

        let command = String::from_utf8_lossy(&buffer[..bytes_read]).to_lowercase();
        println!("Received command: {:?}", command);
        if command.chars().nth(0).is_none() || command.chars().nth(0).unwrap() != '*' {
            return Err(anyhow::anyhow!("Reveived unexpected command"));
        }
        let commands = parse_args(&command);
        println!("Parsed args {:?}", commands);

        match commands[0] {
            "ping" => {
                if commands.len() == 1 {
                    client.write_all(b"+PONG\r\n").await?;
                } else {
                    client
                        .write_all(format!("+{}\r\n", commands[1]).as_bytes())
                        .await?;
                }
            }
            "echo" => {
                if commands.len() < 2 {
                    client
                        .write_all(b"-ERR wrong number of arguments for 'ECHO' command\r\n")
                        .await?;
                } else {
                    client
                        .write_all(format!("+{}\r\n", commands[1]).as_bytes())
                        .await?
                }
            }
            "set" => {
                if commands.len() != 3 {
                    client
                        .write_all(b"-ERR wrong number of arguments for 'SET' command\r\n")
                        .await?;
                    continue;
                } else {
                    let mut data = state.lock().unwrap();
                    data.insert(commands[1].to_string(), commands[2].to_string());
                }
                client.write_all(b"+OK\r\n").await?;
            }
            "get" => {
                if commands.len() != 2 {
                    client
                        .write_all(b"-ERR wrong number of arguments for 'GET' command\r\n")
                        .await?;
                } else {
                    let return_data = {
                        let data = state.lock().unwrap();
                        data.get(commands[1]).cloned()
                    };
                    match return_data {
                        Some(n) => {
                            client
                                .write_all(
                                    format!("${}\r\n{}\r\n", n.len().to_string(), n).as_bytes(),
                                )
                                .await?;
                        }
                        None => {
                            client.write_all(b"$-1\r\n").await?;
                        }
                    }
                }
            }
            _ => {
                client
                    .write_all(b"-ERR unknown command probably not implemented yet\r\n")
                    .await?
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let shared_state: SharedState = Arc::new(Mutex::new(HashMap::new()));
    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    loop {
        let (client, _) = listener.accept().await?;
        let state = Arc::clone(&shared_state);
        tokio::spawn(async move {
            if let Err(e) = handle_client(client, state).await {
                eprintln!("Error handling client: {:?}", e);
            }
        });
    }
}
