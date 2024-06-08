use anyhow::Result;
use redis_rust::{RespHandler, Value};
use std::{collections::HashMap, sync::Arc};
use tokio::net::TcpStream;
use tokio::sync::RwLock;

pub async fn handle_connection(
    socket: TcpStream,
    db_instance: Arc<RwLock<HashMap<String, String>>>,
) -> Result<()> {
    println!("Accepted new connection: {:?}", socket);
    let mut client_handler = RespHandler::new(socket);
    // In a loop, read data from the socket and write the data back.
    loop {
        let value = client_handler.read_value().await?;
        // dbg!(&value);
        println!("Request: {:?}", &value);

        let response = if let Some(value) = value {
            let (command, args) = extract_command(value)?;

            match command.to_lowercase().as_str() {
                "ping" => Value::SimpleString("PONG".to_owned()),
                "echo" => args.first().unwrap().clone(),
                "get" => get_value(&args, &db_instance).await?,
                "set" => set_value(&args, &db_instance).await?,
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
        println!("Resonse: {:?}", &response);

        if let Err(err) = client_handler.write_value(response).await {
            eprintln!("Error writing to socket: {}", err);
            break;
        }
    }
    Ok(())
}

async fn get_value(
    args: &Vec<Value>,
    db_instance: &Arc<RwLock<HashMap<String, String>>>,
) -> Result<Value> {
    // Ensure there is at least one argument
    if args.is_empty() {
        return Err(anyhow::anyhow!("Missing argument for GET command"));
    }

    // Extract the first argument
    let item = match args.first() {
        Some(arg) => arg.clone(),
        None => return Err(anyhow::anyhow!("Missing argument for GET command")),
    };

    // Acquire a read lock on the database instance
    let instance = db_instance.read().await;

    // Match the item to ensure it's a BulkString and get the corresponding value from the database
    let value = match item {
        Value::BulkString(key) => instance.get(&key).cloned(),
        _ => return Err(anyhow::anyhow!("Invalid key type")),
    };

    // Return the found value or an error if the key has no associated value
    match value {
        Some(string) => Ok(Value::BulkString(string)),
        None => Ok(Value::SimpleError("key has no associated value".to_owned())),
    }
}

async fn set_value(
    args: &Vec<Value>,
    db_instance: &Arc<RwLock<HashMap<String, String>>>,
) -> Result<Value> {
    // Ensure there are enough arguments for setting a value
    if args.len() < 2 {
        return Err(anyhow::anyhow!("Not enough arguments for SET command"));
    }

    // Extract the key and value from arguments
    let key = match args.first() {
        Some(Value::BulkString(key)) => key.clone(),
        _ => return Err(anyhow::anyhow!("Invalid key type")),
    };

    let value = match args.get(1) {
        Some(Value::BulkString(value)) => value.clone(),
        _ => return Err(anyhow::anyhow!("Invalid value type")),
    };

    // Acquire a write lock on the database instance
    {
        let mut instance = db_instance.write().await;
        instance.insert(key, value);
    } // The write lock is dropped here

    // Return a success message
    Ok(Value::BulkString("Successfully set value".to_owned()))
}

//"*2\r\n$4\r\nECHO\r\n$3\r\nHEY\r\n"
// return (command, Vec<Argumets>)
pub fn extract_command(value: Value) -> Result<(String, Vec<Value>)> {
    match value {
        Value::Array(array) => Ok((
            unpack_bulk_string(array.first().unwrap().clone())?,
            array.into_iter().skip(1).collect(),
        )),
        _ => return Err(anyhow::anyhow!("Invalid command")),
    }
}

pub fn unpack_bulk_string(value: Value) -> Result<String> {
    match value {
        Value::BulkString(string) => Ok(string),
        _ => return Err(anyhow::anyhow!("Invalid bulk string")),
    }
}
