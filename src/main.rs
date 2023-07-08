mod commands;
mod types;

use anyhow::Result;
use bytes::BytesMut;
use commands::{handle_echo, handle_get, handle_ping, handle_set};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use types::SharedState;

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
            return Err(anyhow::anyhow!("Recived unexpected command"));
        }
        let commands = parse_args(&command);
        println!("Parsed args {:?}", commands);

        match commands[0] {
            "ping" => handle_ping(&mut client, state.clone(), commands).await?,
            "echo" => handle_echo(&mut client, state.clone(), commands).await?,
            "set" => handle_set(&mut client, state.clone(), commands).await?,
            "get" => handle_get(&mut client, state.clone(), commands).await?,
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
