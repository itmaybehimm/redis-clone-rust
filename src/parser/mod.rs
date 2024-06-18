use anyhow::{Context, Result};
use bytes::BytesMut;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub mod tests_parser;
pub enum UserCommand {
    Ping,
    Echo,
    Get,
    Mget,
    Set,
    Del,
    Expire,
    Quit,
    Invalid,
}

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

impl UserCommand {
    pub fn from(command: String) -> Self {
        match command.to_uppercase().as_str() {
            "PING" => Self::Ping,
            "ECHO" => Self::Echo,
            "GET" => Self::Get,
            "MGET" => Self::Mget,
            "SET" => Self::Set,
            "DEL" => Self::Del,
            "EXPIRE" => Self::Expire,
            "QUIT" => Self::Quit,

            _ => Self::Invalid,
        }
    }
}

impl Value {
    pub fn serialize(self) -> String {
        match self {
            Value::SimpleString(s) => format!("+{}\r\n", s),
            Value::BulkString(s) => format!("${}\r\n{}\r\n", s.chars().count(), s),
            Value::Array(arr) => Value::serialize_array(arr),
            Value::SimpleError(s) => format!("-{}\r\n", s),
            _ => panic!("Unsupported value"),
        }
    }

    fn serialize_array(arr: Vec<Value>) -> String {
        let mut serialized = format!("*{}\r\n", arr.len());
        for value in arr {
            serialized.push_str(&value.serialize());
        }
        serialized
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

pub fn parse_message(buffer: BytesMut) -> Result<(Value, usize)> {
    match buffer[0] as char {
        '+' => parse_simple_string(buffer),
        '$' => parse_bulk_string(buffer),
        '*' => parse_array(buffer),
        '-' => parse_simple_error(buffer),
        _ => Err(anyhow::anyhow!("Invalid type {:?}", buffer)),
    }
}

pub fn parse_simple_error(buffer: BytesMut) -> Result<(Value, usize)> {
    if let Some((line, len)) = read_until_crlf(&buffer[1..]) {
        let string = String::from_utf8(line.to_vec()).unwrap();
        Ok((Value::SimpleError(string), len + 1))
    } else {
        Err(anyhow::anyhow!("Invalid string {:?}", buffer))
    }
}

pub fn parse_simple_string(buffer: BytesMut) -> Result<(Value, usize)> {
    if let Some((line, len)) = read_until_crlf(&buffer[1..]) {
        let string = String::from_utf8(line.to_vec()).unwrap();
        Ok((Value::SimpleString(string), len + 1))
    } else {
        Err(anyhow::anyhow!("Invalid string {:?}", buffer))
    }
}

pub fn parse_array(buffer: BytesMut) -> Result<(Value, usize)> {
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

pub fn parse_bulk_string(buffer: BytesMut) -> Result<(Value, usize)> {
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

pub fn read_until_crlf(buffer: &[u8]) -> Option<(&[u8], usize)> {
    for i in 1..buffer.len() {
        if buffer[i - 1] == b'\r' && buffer[i] == b'\n' {
            return Some((&buffer[0..i - 1], i + 1));
        }
    }
    None
}

pub fn parse_int(buffer: &[u8]) -> Result<i64> {
    let string =
        String::from_utf8(buffer.to_vec()).context("Failed to convert buffer to UTF-8 string")?;
    let number = string
        .parse::<i64>()
        .context("Failed to parse string as i64")?;
    Ok(number)
}
