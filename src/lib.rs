use anyhow::{Context, Result};
use bytes::BytesMut;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    SimpleString(String),
    SimpleError(String),
    BulkString(String),
    Array(Vec<Value>),
    InvalidValue,
}

#[derive(Debug)]
pub struct RespHandler {
    pub socket: TcpStream,
    pub buffer: BytesMut,
    pub value: Value,
}

impl Value {
    pub fn serialize(self) -> String {
        match self {
            Value::SimpleString(s) => format!("+{}\r\n", s),
            Value::BulkString(s) => format!("${}\r\n{}\r\n", s.chars().count(), s),
            Value::SimpleError(s) => format!("-{}\r\n", s),
            _ => panic!("Unsupported value"),
        }
    }
}

impl RespHandler {
    pub fn new(socket: TcpStream) -> Self {
        Self {
            socket,
            buffer: BytesMut::with_capacity(512),
            value: Value::InvalidValue,
        }
    }

    pub async fn read_value(&mut self) -> Result<Option<Value>> {
        let bytes_read = self.socket.read_buf(&mut self.buffer).await?;
        if bytes_read == 0 {
            return Ok(None);
        }
        // dbg!(&self.buffer);
        let (value, _) = parse_message(self.buffer.split())?;
        Ok(Some(value))
    }

    pub async fn write_value(&mut self, value: Value) -> Result<()> {
        // dbg!(&value);
        self.socket.write_all(value.serialize().as_bytes()).await?;
        Ok(())
    }
}

fn parse_message(buffer: BytesMut) -> Result<(Value, usize)> {
    match buffer[0] as char {
        '+' => parse_simple_string(buffer),
        '$' => parse_bulk_string(buffer),
        '*' => parse_array(buffer),
        '-' => parse_simple_error(buffer),
        _ => Err(anyhow::anyhow!("Invalid type {:?}", buffer)),
    }
}

fn parse_simple_error(buffer: BytesMut) -> Result<(Value, usize)> {
    if let Some((line, len)) = read_until_crlf(&buffer[1..]) {
        let string = String::from_utf8(line.to_vec()).unwrap();
        Ok((Value::SimpleError(string), len + 1))
    } else {
        Err(anyhow::anyhow!("Invalid string {:?}", buffer))
    }
}

fn parse_simple_string(buffer: BytesMut) -> Result<(Value, usize)> {
    if let Some((line, len)) = read_until_crlf(&buffer[1..]) {
        let string = String::from_utf8(line.to_vec()).unwrap();
        Ok((Value::SimpleString(string), len + 1))
    } else {
        Err(anyhow::anyhow!("Invalid string {:?}", buffer))
    }
}

fn parse_array(buffer: BytesMut) -> Result<(Value, usize)> {
    let (array_length, mut bytes_consumed) =
        if let Some((line, len)) = read_until_crlf(&buffer[1..]) {
            let array_length = parse_int(line).unwrap();
            (array_length, len + 1)
        } else {
            return Err(anyhow::anyhow!("Invalid array {:?}", buffer));
        };

    let mut items = vec![];

    for _ in 0..array_length {
        let (array_item, length) = parse_message(BytesMut::from(&buffer[bytes_consumed..]))?;
        bytes_consumed += length;
        items.push(array_item);
    }

    Ok((Value::Array(items), bytes_consumed))
}

fn parse_bulk_string(buffer: BytesMut) -> Result<(Value, usize)> {
    let (string_length, bytes_consumed) = if let Some((line, len)) = read_until_crlf(&buffer[1..]) {
        let string_length = parse_int(line).unwrap();
        (string_length, len + 1)
    } else {
        return Err(anyhow::anyhow!("Invalid bulk string {:?}", buffer));
    };

    let end_of_bulk_string = bytes_consumed + string_length as usize;
    let total_parsed = end_of_bulk_string + 2;

    let string = String::from_utf8(buffer[bytes_consumed..end_of_bulk_string].to_vec())
        .context("Invalid bulk string")?;
    Ok((Value::BulkString(string), total_parsed))
}

fn read_until_crlf(buffer: &[u8]) -> Option<(&[u8], usize)> {
    for i in 1..buffer.len() {
        if buffer[i - 1] == b'\r' && buffer[i] == b'\n' {
            return Some((&buffer[0..i - 1], i + 1));
        }
    }
    None
}

fn parse_int(buffer: &[u8]) -> Result<i64> {
    let string =
        String::from_utf8(buffer.to_vec()).context("Failed to convert buffer to UTF-8 string")?;
    let number = string
        .parse::<i64>()
        .context("Failed to parse string as i64")?;
    Ok(number)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    #[tokio::test]
    async fn test_simple_string_serialization() {
        let value = Value::SimpleString("OK".to_string());
        assert_eq!(value.serialize(), "+OK\r\n");
    }

    #[tokio::test]
    async fn test_bulk_string_serialization() {
        let value = Value::BulkString("foobar".to_string());
        assert_eq!(value.serialize(), "$6\r\nfoobar\r\n");
    }

    #[tokio::test]
    async fn test_simple_error_serialization() {
        let value = Value::SimpleError("Error message".to_string());
        assert_eq!(value.serialize(), "-Error message\r\n");
    }

    #[tokio::test]
    async fn test_read_simple_string() -> Result<()> {
        let (client, mut server) = create_client_server().await?;

        server.write_all(b"+OK\r\n").await?;
        let mut handler = RespHandler::new(client);
        let value = handler.read_value().await?.unwrap();
        assert_eq!(value, Value::SimpleString("OK".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn test_read_bulk_string() -> Result<()> {
        let (client, mut server) = create_client_server().await?;
        server.write_all(b"$6\r\nfoobar\r\n").await?;
        let mut handler = RespHandler::new(client);
        let value = handler.read_value().await?.unwrap();
        assert_eq!(value, Value::BulkString("foobar".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn test_read_simple_error() -> Result<()> {
        let (client, mut server) = create_client_server().await?;
        server.write_all(b"-Error message\r\n").await?;
        let mut handler = RespHandler::new(client);
        let value = handler.read_value().await?.unwrap();
        assert_eq!(value, Value::SimpleError("Error message".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn test_write_simple_string() -> Result<()> {
        let (client, mut server) = create_client_server().await?;
        let mut handler = RespHandler::new(client);
        handler
            .write_value(Value::SimpleString("OK".to_string()))
            .await?;
        let mut buffer = vec![0; 5];
        server.read_exact(&mut buffer).await?;
        assert_eq!(buffer, b"+OK\r\n");
        Ok(())
    }

    #[tokio::test]
    async fn test_write_bulk_string() -> Result<()> {
        let (client, mut server) = create_client_server().await?;
        let mut handler = RespHandler::new(client);
        handler
            .write_value(Value::BulkString("foobar".to_string()))
            .await?;

        let mut buffer = vec![0; 12];

        server.read_exact(&mut buffer).await?;
        dbg!("await pass");

        assert_eq!(buffer, b"$6\r\nfoobar\r\n");
        Ok(())
    }

    #[tokio::test]
    async fn test_write_simple_error() -> Result<()> {
        let (client, mut server) = create_client_server().await?;
        let mut handler = RespHandler::new(client);
        handler
            .write_value(Value::SimpleError("Error message".to_string()))
            .await?;
        let mut buffer = vec![0; 16];
        server.read_exact(&mut buffer).await?;
        assert_eq!(buffer, b"-Error message\r\n");
        Ok(())
    }

    async fn create_client_server() -> Result<(TcpStream, TcpStream)> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let client = TcpStream::connect(addr).await?;
        let (server, _) = listener.accept().await?;
        Ok((client, server))
    }
}
