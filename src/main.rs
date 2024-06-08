mod connection;

use std::{collections::HashMap, sync::Arc};

use connection::handle_connection;

use anyhow::Result;
use tokio::{net::TcpListener, sync::RwLock};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    let db: Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new()));
    loop {
        let (socket, _) = listener.accept().await?;
        let instance = Arc::clone(&db);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, instance).await {
                eprintln!("Failed to handle connection: {}", e);
            }
        });
    }
}
