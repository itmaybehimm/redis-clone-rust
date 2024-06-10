use anyhow::Result;

use redis_rust::{RespHandler, Value};
use tokio::time::{self, sleep};

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

        let response = if let Some(value) = value {
            let (command, args) = extract_command(value)?;

            match command.to_lowercase().as_str() {
                "ping" => Value::SimpleString("PONG".to_owned()),
                "echo" => args.first().unwrap().clone(),
                "get" => get_value(&args, &db_instance).await?,
                "mget" => mget_value(&args, &db_instance).await?,
                "expire" => expire_value(&args, &db_instance).await?,
                "set" => set_value(&args, &db_instance).await?,
                "del" => del_value(&args, &db_instance).await?,
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

async fn mget_value(
    args: &Vec<Value>,
    db_instance: &Arc<RwLock<HashMap<String, String>>>,
) -> Result<Value> {
    // Ensure there is at least one argument
    if args.is_empty() {
        return Ok(Value::SimpleError(
            "Missing argument for MGET command".to_owned(),
        ));
    }

    // Acquire a read lock on the database instance
    let instance = db_instance.read().await;

    let mut result = Vec::new();

    // Match the item to ensure it's a BulkString and get the corresponding value from the database
    for value in args.iter() {
        let key = match value {
            Value::BulkString(key) => key.clone(),
            _ => {
                return Ok(Value::SimpleError(
                    "One or more keys are invalid".to_owned(),
                ))
            }
        };

        let value = match instance.get(&key) {
            Some(string) => Value::BulkString(string.clone()),
            None => Value::SimpleError(format!("The key {} has no associated value", key)),
        };

        result.push(value);
    }

    Ok(Value::Array(result))
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

    let result;

    // Acquire a write lock on the database instance
    {
        let mut instance = db_instance.write().await;
        result = instance.insert(key, value);
    } // The write lock is dropped here

    match result {
        Some(_) => Ok(Value::BulkString(
            "Scucessfully updated value in database".to_owned(),
        )),
        None => Ok(Value::BulkString(
            "Scucessfully inserted value in database".to_owned(),
        )),
    }
}

async fn del_value(
    args: &Vec<Value>,
    db_instance: &Arc<RwLock<HashMap<String, String>>>,
) -> Result<Value> {
    // Ensure there are enough arguments for setting a value
    if args.len() < 1 {
        return Err(anyhow::anyhow!("Not enough arguments for DEL command"));
    }

    // Extract the key and value from arguments
    let key = match args.first() {
        Some(Value::BulkString(key)) => key.clone(),
        _ => return Err(anyhow::anyhow!("Invalid key type")),
    };

    let value;

    {
        let mut instace = db_instance.write().await;
        value = instace.remove(&key);
    }

    match value {
        Some(_) => Ok(Value::BulkString(
            "Removed the key from database".to_owned(),
        )),
        None => Ok(Value::SimpleError(
            "Value doesnt exist in database".to_owned(),
        )),
    }
}

async fn expire_value(
    args: &Vec<Value>,
    db_instance: &Arc<RwLock<HashMap<String, String>>>,
) -> Result<Value> {
    if args.len() != 2 {
        return Ok(Value::SimpleError("Invalid number of arguments".to_owned()));
    }

    if let (Some(Value::BulkString(key)), Some(Value::BulkString(seconds))) =
        (args.get(0), args.get(1))
    {
        let seconds = seconds.parse();
        let seconds = match seconds {
            Ok(s) => s,
            Err(_) => return Ok(Value::SimpleError("Invalid number of seconds".to_owned())),
        };
        let seconds = time::Duration::from_secs(seconds);

        let key_clone = key.clone();
        let db_instance_clone = Arc::clone(db_instance);

        tokio::spawn(async move {
            sleep(seconds).await;
            let mut instance = db_instance_clone.write().await;
            instance.remove(&key_clone);
            println!("removed: {} after {:?} seconds", key_clone, seconds);
        });
        Ok(Value::SimpleString("OK".to_owned()))
    } else {
        Ok(Value::SimpleError("Invalid arguments".to_owned()))
    }
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
