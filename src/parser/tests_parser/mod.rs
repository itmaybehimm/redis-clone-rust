#[cfg(test)]
mod tests {
    use super::super::*;
    use anyhow::Result;
    use bytes::BytesMut;

    #[test]
    fn test_serialize_simple_string() -> Result<()> {
        let value = Value::SimpleString("OK".to_string());
        assert_eq!(value.serialize(), "+OK\r\n");
        Ok(())
    }

    #[test]
    fn test_serialize_bulk_string() -> Result<()> {
        let value = Value::BulkString("foobar".to_string());
        assert_eq!(value.serialize(), "$6\r\nfoobar\r\n");
        Ok(())
    }

    #[test]
    fn test_serialize_simple_error() -> Result<()> {
        let value = Value::SimpleError("Error message".to_string());
        assert_eq!(value.serialize(), "-Error message\r\n");
        Ok(())
    }

    #[test]
    fn test_serialize_array() -> Result<()> {
        let value = Value::Array(vec![
            Value::SimpleString("foo".to_string()),
            Value::BulkString("bar".to_string()),
        ]);
        assert_eq!(value.serialize(), "*2\r\n+foo\r\n$3\r\nbar\r\n");
        Ok(())
    }

    #[test]
    fn test_parse_simple_string() -> Result<()> {
        let buffer = BytesMut::from("+OK\r\n");
        let (value, _) = parse_simple_string(buffer)?;
        assert_eq!(value, Value::SimpleString("OK".to_string()));
        Ok(())
    }

    #[test]
    fn test_parse_bulk_string() -> Result<()> {
        let buffer = BytesMut::from("$6\r\nfoobar\r\n");
        let (value, _) = parse_bulk_string(buffer)?;
        assert_eq!(value, Value::BulkString("foobar".to_string()));
        Ok(())
    }

    #[test]
    fn test_parse_simple_error() -> Result<()> {
        let buffer = BytesMut::from("-Error message\r\n");
        let (value, _) = parse_simple_error(buffer)?;
        assert_eq!(value, Value::SimpleError("Error message".to_string()));
        Ok(())
    }

    #[test]
    fn test_parse_array() -> Result<()> {
        let buffer = BytesMut::from("*2\r\n+foo\r\n$3\r\nbar\r\n");
        let (value, _) = parse_array(buffer)?;
        assert_eq!(
            value,
            Value::Array(vec![
                Value::SimpleString("foo".to_string()),
                Value::BulkString("bar".to_string()),
            ])
        );
        Ok(())
    }
}

#[cfg(test)]
mod client_server_tests {
    use super::super::*;
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
