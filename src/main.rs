use anyhow::Result;
use bytes::BytesMut;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{Instant, Duration};

#[derive(Clone, Debug)]
struct ValueWithExpiry {
    value: String,
    expiry: Option<Duration>,
    insert_time: Instant,
}

type SharedState = Arc<Mutex<HashMap<String, ValueWithExpiry>>>;

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
                if commands.len() != 3 && commands.len() != 5 {
                    client
                        .write_all(b"-ERR wrong number of arguments for 'SET' command\r\n")
                        .await?;
                    continue;
                } else {
                    let value: ValueWithExpiry;
                    if commands.len() == 5 {
                        value =  ValueWithExpiry {
                            value: commands[2].to_string(),
                            insert_time: Instant::now(),
                            expiry: Some(Duration::from_millis(commands[4].parse::<u64>().unwrap()))

                        };
                    } else {
                        value = ValueWithExpiry {
                            value: commands[2].to_string(),
                            insert_time: Instant::now(),
                            expiry: None
                        };
                    }
                    let mut data = state.lock().unwrap();
                    println!("Inserting data {:?}", value);
                    data.insert(commands[1].to_string(), value);
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
                    println!("Retrived data from db {:?}", return_data);
                    match return_data {
                        Some(n) => {
                            if n.expiry.is_some() && n.insert_time + n.expiry.unwrap() < tokio::time::Instant::now() {
                                {
                                    println!("Data was to old and is being removed, {:?}", n);
                                    let mut data = state.lock().unwrap();
                                    data.remove(commands[1]);
                                }
                                client.write_all(b"$-1\r\n").await?;
                                continue;
                            }

                            client
                                .write_all(
                                    format!("${}\r\n{}\r\n", n.value.len().to_string(), n.value).as_bytes(),
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
