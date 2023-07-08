use crate::types::{SharedState, ValueWithExpiry};
use anyhow::Result;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::{Duration, Instant};

pub async fn handle_ping(
    client: &mut TcpStream,
    _state: SharedState,
    commands: Vec<&str>,
) -> Result<()> {
    if commands.len() == 1 {
        client.write_all(b"+PONG\r\n").await?;
    } else {
        client
            .write_all(format!("+{}\r\n", commands[1]).as_bytes())
            .await?;
    }
    return Ok(());
}

pub async fn handle_echo(
    client: &mut TcpStream,
    _state: SharedState,
    commands: Vec<&str>,
) -> Result<()> {
    if commands.len() < 2 {
        client
            .write_all(b"-ERR wrong number of arguments for 'ECHO' command\r\n")
            .await?;
    } else {
        client
            .write_all(format!("+{}\r\n", commands[1]).as_bytes())
            .await?
    }
    return Ok(());
}

pub async fn handle_set(
    client: &mut TcpStream,
    _state: SharedState,
    commands: Vec<&str>,
) -> Result<()> {
    if commands.len() != 3 && commands.len() != 5 {
        client
            .write_all(b"-ERR wrong number of arguments for 'SET' command\r\n")
            .await?;
        return Err(anyhow::anyhow!(
            "wrong number of arguments for 'SET' command"
        ));
    } else {
        let value: ValueWithExpiry;
        if commands.len() == 5 {
            if commands[3] == "px" {
                value = ValueWithExpiry {
                    value: commands[2].to_string(),
                    insert_time: Instant::now(),
                    expiry: Some(Duration::from_millis(commands[4].parse::<u64>().unwrap())),
                };
            } else if commands[3] == "ex" {
                value = ValueWithExpiry {
                    value: commands[2].to_string(),
                    insert_time: Instant::now(),
                    expiry: Some(Duration::from_secs(commands[4].parse::<u64>().unwrap())),
                }
            } else {
                client
                    .write_all(b"-ERR wrong number of arguments for 'SET' command\r\n")
                    .await?;
                return Err(anyhow::anyhow!(
                    "wrong number of arguments for 'SET' command"
                ));
            }
        } else {
            value = ValueWithExpiry {
                value: commands[2].to_string(),
                insert_time: Instant::now(),
                expiry: None,
            };
        }
        let mut data = _state.lock().unwrap();
        println!("Inserting data {:?}", value);
        data.insert(commands[1].to_string(), value);
    }
    client.write_all(b"+OK\r\n").await?;
    return Ok(());
}

pub async fn handle_get(
    client: &mut TcpStream,
    _state: SharedState,
    commands: Vec<&str>,
) -> Result<()> {
    if commands.len() != 2 {
        client
            .write_all(b"-ERR wrong number of arguments for 'GET' command\r\n")
            .await?;
    } else {
        let return_data = {
            let data = _state.lock().unwrap();
            data.get(commands[1]).cloned()
        };
        println!("Retrived data from db {:?}", return_data);
        match return_data {
            Some(n) => {
                if n.expiry.is_some() && n.insert_time + n.expiry.unwrap() < Instant::now() {
                    {
                        println!("Data was to old and is being removed, {:?}", n);
                        let mut data = _state.lock().unwrap();
                        data.remove(commands[1]);
                    }
                    client.write_all(b"$-1\r\n").await?;
                    return Ok(());
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
    return Ok(());
}
