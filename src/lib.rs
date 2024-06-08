use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum Command {
    SimpleString,
    SimpleError,
    BulkString,
    Array,
    Integer,
    InvalidCommand,
}

#[derive(Debug, PartialEq)]
pub struct ClientRequest {
    pub command: Command,
    pub data: String,
    pub elements: Vec<String>,
}

impl ClientRequest {
    pub fn from(message: &str) -> Self {
        // Check if the message is empty to avoid panic
        let command = if message.is_empty() {
            Command::InvalidCommand
        } else {
            match message.chars().next().unwrap() {
                '+' => Command::SimpleString,
                '-' => Command::SimpleError,
                ':' => Command::Integer,
                '$' => Command::BulkString,
                '*' => Command::Array,
                _ => Command::InvalidCommand,
            }
        };

        let data = String::from(message);
        let elements = match command {
            Command::Array => parse_array_elements(message),
            Command::SimpleString => parse_simple_string(message),
            Command::BulkString => parse_bulk_string(message),
            Command::Integer => parse_integer(message),
            _ => vec![],
        };

        Self {
            command,
            data,
            elements,
        }
    }
}

//simple string in formart +(str)\r\n
fn parse_simple_string(message: &str) -> Vec<String> {
    let mut elements = Vec::new();
    let line = message.trim_end();
    elements.push(line[1..].to_string());
    elements
}

//bulk string in formart $(len)\r\n(str)\r\n
fn parse_bulk_string(message: &str) -> Vec<String> {
    let mut elements = Vec::new();
    let mut lines = message.lines();
    while let Some(line) = lines.next() {
        if let Some(len) = usize::from_str(&line[1..]).ok() {
            if let Some(data) = lines.next() {
                if data.len() == len {
                    elements.push(data.to_string());
                }
            }
        }
    }
    elements
}

//array in format *(sizearray)\r\n(elements)
fn parse_array_elements(message: &str) -> Vec<String> {
    let mut elements = Vec::new();
    let mut lines = message.lines();

    lines.next(); // Skip the array prefix (e.g., *2)
    while let Some(line) = lines.next() {
        if line.starts_with('$') {
            let next_line = lines.next().expect("Invalid bulk string");
            let combined_line = format!("{}\n{}", line, next_line);

            let parsed_bulk_string = parse_bulk_string(&combined_line);

            for bulk_string in parsed_bulk_string {
                elements.push(bulk_string);
            }
        }
    }
    elements
}

fn parse_integer(message: &str) -> Vec<String> {
    let mut elements = Vec::new();
    let line = message.trim_end();
    elements.push(line[1..].to_string());
    elements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_string() {
        let message = "+OK\r\n";
        let request = ClientRequest::from(message);
        assert_eq!(request.command, Command::SimpleString);
        assert_eq!(request.data, message);
        assert_eq!(request.elements, vec!["OK".to_string()]);
    }

    #[test]
    fn test_parse_error() {
        let message = "-Error message\r\n";
        let request = ClientRequest::from(message);
        assert_eq!(request.command, Command::SimpleError);
        assert_eq!(request.data, message);
        assert!(request.elements.is_empty());
    }

    #[test]
    fn test_parse_bulk_string() {
        let message = "$6\r\nfoobar\r\n";
        let request = ClientRequest::from(message);
        assert_eq!(request.command, Command::BulkString);
        assert_eq!(request.data, message);
        assert_eq!(request.elements, vec!["foobar".to_string()]);
    }

    #[test]
    fn test_parse_array() {
        let message = "*2\r\n$7\r\nCOMMAND\r\n$4\r\nDOCS\r\n";
        let request = ClientRequest::from(message);
        assert_eq!(request.command, Command::Array);
        assert_eq!(request.data, message);
        assert_eq!(
            request.elements,
            vec!["COMMAND".to_string(), "DOCS".to_string()]
        );
    }

    #[test]
    fn test_parse_invalid_command() {
        let message = "invalid message";
        let request = ClientRequest::from(message);
        assert_eq!(request.command, Command::InvalidCommand);
        assert_eq!(request.data, message);
        assert!(request.elements.is_empty());
    }

    #[test]
    fn test_parse_empty_message() {
        let message = "";
        let request = ClientRequest::from(message);
        assert_eq!(request.command, Command::InvalidCommand);
        assert_eq!(request.data, message);
        assert!(request.elements.is_empty());
    }

    #[test]
    fn test_parse_array_with_single_element() {
        let message = "*1\r\n$4\r\nPING\r\n";
        let request = ClientRequest::from(message);
        assert_eq!(request.command, Command::Array);
        assert_eq!(request.data, message);
        assert_eq!(request.elements, vec!["PING".to_string()]);
    }

    #[test]
    fn test_parse_integer() {
        let message = ":1000\r\n";
        let request = ClientRequest::from(message);
        assert_eq!(request.command, Command::Integer);
        assert_eq!(request.data, message);
        assert_eq!(request.elements, vec!["1000".to_string()]);
    }
}
