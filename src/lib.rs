use anyhow::{Context, Ok, Result};
use bytes::BytesMut;
use tokio::{io::AsyncReadExt, net::TcpStream};
#[derive(Debug, PartialEq)]
pub enum Value {
    SimpleString(String),
    BulkString(String),
    Array(Vec<Value>),
    InvalidValue,
}

#[derive(Debug)]
pub struct ClientHandler {
    pub socket: TcpStream,
    pub buffer: BytesMut,
    pub value: Value,
}

impl Value {
    pub fn serialize(self) -> String {
        match self {
            Value::SimpleString(s) => format!("+{}\r\n", s),
            Value::BulkString(s) => format!("${}\r\n{}\r\n", s.chars().count(), s),
            _ => panic!("Unsupported value"),
        }
    }
}

impl ClientHandler {
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

        let (value, _) = parse_message(self.buffer.split())?;

        Ok(Some(value))
    }

    pub async fn write_value(&mut self) {
        //TODO
    }
}

fn parse_message(buffer: BytesMut) -> Result<(Value, usize)> {
    match buffer[0] as char {
        '+' => return parse_simple_string(buffer),
        '$' => return parse_bulk_string(buffer),
        '*' => return parse_array(buffer),
        _ => return Err(anyhow::anyhow!("Invalid type {:?}", buffer)),
    };
}

fn parse_simple_string(buffer: BytesMut) -> Result<(Value, usize)> {
    //skip the +
    if let Some((line, len)) = read_until_crfl(&buffer[1..]) {
        let string = String::from_utf8(line.to_vec()).unwrap();

        //what next character should be indexed from since we skipped + len+1
        return Ok((Value::SimpleString(string), len + 1));
    } else {
        return Err(anyhow::anyhow!("Invalid string {:?}", buffer));
    }
}

fn parse_array(buffer: BytesMut) -> Result<(Value, usize)> {
    //first line *(len)
    // say *2\r\n.....
    // we read from 2 to \n so consume 3 bytes + 1 * we skipped
    let (array_length, mut bytes_consumed) =
        if let Some((line, len)) = read_until_crfl(&buffer[1..]) {
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

    return Ok((Value::Array(items), bytes_consumed));
}

fn parse_bulk_string(buffer: BytesMut) -> Result<(Value, usize)> {
    //first line $(len)
    // say $2\r\n.....
    // we read from 2 to \n so consume 3 bytes + 1 * we skipped
    let (string_length, bytes_consumed) = if let Some((line, len)) = read_until_crfl(&buffer[1..]) {
        let string_length = parse_int(line).unwrap();
        (string_length, len + 1)
    } else {
        return Err(anyhow::anyhow!("Invalid bulk string {:?}", buffer));
    };

    let end_of_bulk_string = bytes_consumed + string_length as usize;
    let total_parsed = end_of_bulk_string + 2;

    let string = String::from_utf8(buffer[bytes_consumed..end_of_bulk_string].to_vec())
        .context("Invalid bulk string")?;
    return Ok((Value::BulkString(string), total_parsed));
}

fn read_until_crfl(buffer: &[u8]) -> Option<(&[u8], usize)> {
    for i in 1..buffer.len() {
        if buffer[i - 1] == b'\r' && buffer[i] == b'\n' {
            // say abc\r\nab\r\n is buffer so upto \n is i=4 which consumes 5 bytes
            return Some((&buffer[0..i], i + 1));
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
