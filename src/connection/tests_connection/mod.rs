#[cfg(test)]
mod tests {
    use super::super::*;
    use std::{collections::HashMap, sync::Arc};
    use tokio::net::TcpListener;
    use tokio::sync::RwLock;

    async fn setup() -> (TcpStream, Arc<RwLock<HashMap<String, String>>>) {
        // Start a TCP listener
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Create a shared database instance
        let db_instance = Arc::new(RwLock::new(HashMap::new()));

        // Spawn a task to accept connections
        let db_instance_clone = Arc::clone(&db_instance);
        tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            handle_connection(socket, db_instance_clone).await.unwrap();
        });

        // Connect to the listener
        let socket = TcpStream::connect(addr).await.unwrap();

        (socket, db_instance)
    }

    #[tokio::test]
    async fn test_ping_command() {
        let (socket, _) = setup().await;
        let mut client_handler = RespHandler::new(socket);

        // Send the PING command
        client_handler
            .write_value(Value::Array(vec![Value::BulkString("PING".to_owned())]))
            .await
            .unwrap();

        // Read the response
        let response = client_handler.read_value().await.unwrap().unwrap();
        assert_eq!(response, Value::SimpleString("PONG".to_owned()));
    }

    #[tokio::test]
    async fn test_echo_command() {
        let (socket, _) = setup().await;
        let mut client_handler = RespHandler::new(socket);

        // Send the ECHO command
        client_handler
            .write_value(Value::Array(vec![
                Value::BulkString("ECHO".to_owned()),
                Value::BulkString("Hello, World!".to_owned()),
            ]))
            .await
            .unwrap();

        // Read the response
        let response = client_handler.read_value().await.unwrap().unwrap();
        assert_eq!(response, Value::BulkString("Hello, World!".to_owned()));
    }

    #[tokio::test]
    async fn test_set_and_get_command() {
        let (socket, _db_instance) = setup().await;
        let mut client_handler = RespHandler::new(socket);

        // Send the SET command
        client_handler
            .write_value(Value::Array(vec![
                Value::BulkString("SET".to_owned()),
                Value::BulkString("key".to_owned()),
                Value::BulkString("value".to_owned()),
            ]))
            .await
            .unwrap();

        // Read the response
        let response = client_handler.read_value().await.unwrap().unwrap();
        assert_eq!(
            response,
            Value::BulkString("Scucessfully inserted value in database".to_owned())
        );

        // Send the GET command
        client_handler
            .write_value(Value::Array(vec![
                Value::BulkString("GET".to_owned()),
                Value::BulkString("key".to_owned()),
            ]))
            .await
            .unwrap();

        // Read the response
        let response = client_handler.read_value().await.unwrap().unwrap();
        assert_eq!(response, Value::BulkString("value".to_owned()));
    }

    #[tokio::test]
    async fn test_del_command() {
        let (socket, db_instance) = setup().await;
        let mut client_handler = RespHandler::new(socket);

        // First, set a key
        client_handler
            .write_value(Value::Array(vec![
                Value::BulkString("SET".to_owned()),
                Value::BulkString("key".to_owned()),
                Value::BulkString("value".to_owned()),
            ]))
            .await
            .unwrap();
        client_handler.read_value().await.unwrap().unwrap();

        // Send the DEL command
        client_handler
            .write_value(Value::Array(vec![
                Value::BulkString("DEL".to_owned()),
                Value::BulkString("key".to_owned()),
            ]))
            .await
            .unwrap();

        // Read the response
        let response = client_handler.read_value().await.unwrap().unwrap();
        assert_eq!(
            response,
            Value::BulkString("Removed the key from database".to_owned())
        );

        // Verify the key is deleted
        let db_instance = db_instance.read().await;
        assert!(db_instance.get("key").is_none());
    }

    #[tokio::test]
    async fn test_expire_command() {
        let (socket, db_instance) = setup().await;
        let mut client_handler = RespHandler::new(socket);

        // First, set a key
        client_handler
            .write_value(Value::Array(vec![
                Value::BulkString("SET".to_owned()),
                Value::BulkString("key".to_owned()),
                Value::BulkString("value".to_owned()),
            ]))
            .await
            .unwrap();
        client_handler.read_value().await.unwrap().unwrap();

        // Send the EXPIRE command
        client_handler
            .write_value(Value::Array(vec![
                Value::BulkString("EXPIRE".to_owned()),
                Value::BulkString("key".to_owned()),
                Value::BulkString("1".to_owned()),
            ]))
            .await
            .unwrap();

        // Read the response
        let response = client_handler.read_value().await.unwrap().unwrap();
        assert_eq!(response, Value::SimpleString("OK".to_owned()));

        // Wait for more than 1 second to ensure the key expires
        sleep(tokio::time::Duration::from_secs(2)).await;

        // Verify the key is deleted
        let db_instance = db_instance.read().await;
        assert!(db_instance.get("key").is_none());
    }
}
