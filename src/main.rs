use anyhow::{Ok, Result};
use tokio::net::{TcpListener, TcpStream};

use redis_rust::ClientHandler;
use redis_rust::Value;

async fn handle_connection(socket: TcpStream) -> Result<()> {
    println!("Accepted new connection: {:?}", socket);
    let mut client_handler = ClientHandler::new(socket);
    // In a loop, read data from the socket and write the data back.
    loop {
        let value = client_handler.read_value().await?;
        dbg!(&value);
        let response = if let Some(value) = value {
            let (command, args) = extract_command(value)?;

            match command.to_lowercase().as_str() {
                "ping" => Value::SimpleString("PONG".to_owned()),
                "echo" => args.first().unwrap().clone(),
                "quit" => {
                    println!("Client requested to quit.");
                    break;
                }
                _ => Value::SimpleError("Invalid command".to_owned()),
            }
        } else {
            println!("Client requested to quit.");
            break;
        };
        if let Err(err) = client_handler.write_value(response).await {
            eprintln!("Error writing to socket: {}", err);
            break;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket).await {
                eprintln!("Failed to handle connection: {}", e);
            }
        });
    }
}

//"*2\r\n$4\r\nECHO\r\n$3\r\nHEY\r\n"
// return (command, Vec<Argumets>)
fn extract_command(value: Value) -> Result<(String, Vec<Value>)> {
    match value {
        Value::Array(array) => Ok((
            unpack_bulk_string(array.first().unwrap().clone())?,
            array.into_iter().skip(1).collect(),
        )),
        _ => return Err(anyhow::anyhow!("Invalid command")),
    }
}

fn unpack_bulk_string(value: Value) -> Result<String> {
    match value {
        Value::BulkString(string) => Ok(string),
        _ => return Err(anyhow::anyhow!("Invalid bulk string")),
    }
}
