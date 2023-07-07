use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use anyhow::Result;
use bytes::BytesMut;

fn parse_args(command: &str) -> Vec<&str> {
    let lines: Vec<&str> = command.split("\r\n").collect();
    let mut commands_and_args: Vec<&str> = vec![];

    let mut i = 0;
    while i < lines.len() {
        if lines[i].starts_with('*') {
            i += 1;
            continue;
        }
        else if lines[i].starts_with('$') {
            i += 1;
            if i < lines.len() {
                commands_and_args.push(lines[i]);
            }
        }
        i += 1;
    }
    return commands_and_args;
}

async fn handle_client(mut client: TcpStream) -> Result<()> {
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
            return Err(anyhow::anyhow!("Reveived unexpected command"))
        }
        let commands = parse_args(&command);
        println!("Parsed args {:?}", commands);

        match commands[0] {
            "ping" => {
                if commands.len() == 1 {
                    client.write_all(b"+PONG\r\n").await?;
                }
                else {
                    client.write_all(format!("+{}\r\n", commands[1]).as_bytes()).await?;
                }
            }
            "echo" => {
                if commands.len() < 2 {
                    client.write_all(b"-ERR wrong number of arguments for 'ECHO' command\r\n").await?;
                }
                else {
                    client.write_all(format!("+{}\r\n", commands[1]).as_bytes()).await?
                }
            }
            _ => {client.write_all(b"-ERR unknown command probably not implemented yet\r\n").await?}
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    loop {
        let (client, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = handle_client(client).await {
                eprintln!("Error handling client: {:?}", e);
            }
        });
    }
}
