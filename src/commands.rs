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
    if commands.len() > 2 {
        client
            .write_all(b"-ERR wrong number of arguments for 'PING' command\r\n")
            .await?;
        eprintln!("ERR wrong number of arguments for 'PING' command")
    }
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
    if commands.len() != 2 {
        client
            .write_all(b"-ERR wrong number of arguments for 'ECHO' command\r\n")
            .await?;
        eprintln!("ERR wrong number of arguments for 'ECHO' command")
    } else {
        client
            .write_all(format!("+{}\r\n", commands[1]).as_bytes())
            .await?
    }
    return Ok(());
}

pub async fn handle_set(
    client: &mut TcpStream,
    state: SharedState,
    commands: Vec<&str>,
) -> Result<()> {
    {
        let value: ValueWithExpiry;
        let mut skip_next = false;
        if !check_set_command_syntax(&commands) {
            client.write_all(b"-ERR syntax error\r\n").await?;
            return Ok(());
        }
        let mut expiry: Option<Duration> = None;
        let mut only_if_new = false;
        let mut only_if_exists = false;
        for (index, command) in commands.iter().enumerate() {
            if index < 3 {
                continue
            }
            if skip_next {
                skip_next = false;
                continue;
            }
            match command {
                &"px" => {
                    skip_next = true;
                    expiry = Some(Duration::from_millis(commands[index+1].parse::<u64>().unwrap()));
                },
                &"ex" => {
                    skip_next = true;
                    expiry = Some(Duration::from_secs(commands[index+1].parse::<u64>().unwrap()))
                },
                &"nx" => {
                    only_if_new = true;
                },
                &"xx" => {
                    only_if_exists = true;
                }
                _ => {
                    client.write_all(b"-ERR syntax error\r\n").await?;
                    return Ok(())
                }
            }
        }
        {
            let data = {
                let data = state.lock().unwrap();
                data.get(commands[1]).cloned()
            };
            if only_if_exists && data.is_none() {
                client.write_all(b"$-1\r\n").await?;
                return Ok(());
            }
            if only_if_new && data.is_some() {
                client.write_all(b"$-1\r\n").await?;
                return Ok(());
            }
        }
        value = ValueWithExpiry {
            value: commands[2].to_string(),
            insert_time: Instant::now(),
            expiry,
        };
        let mut data = state.lock().unwrap();
        println!("Inserting data {:?}", value);
        data.insert(commands[1].to_string(), value);
    }
    client.write_all(b"+OK\r\n").await?;
    return Ok(());
}

fn check_set_command_syntax(commands: &Vec<&str>) -> bool {
    let mut px_set = false;
    let mut ex_set = false;
    let mut xx_set = false;
    let mut nx_set = false;

    for i in 0..commands.len() {
        match commands[i] {
            "px" => {
                if i < 3 || i +1  == commands.len()  {
                    return false;
                }
                if ex_set {
                    return false;
                };
                px_set = true;
            }
            "ex" => {
                if i < 3 || i +1  == commands.len() {
                    return false;
                }
                if px_set {
                    return false;
                }
                ex_set = true;
            }
            "xx" => {
                if i < 3 {
                    return false;
                }
                if nx_set {
                    return false;
                }
                xx_set = true;
            }
            "nx" => {
                if i < 3 {
                    return false;
                }
                if xx_set {
                    return false;
                }
                nx_set = true;
            }
            _ => continue,
        }
    }

    return true;
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
