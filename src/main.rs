use anyhow::Result;
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use redis_rust::ClientHandler;

async fn handle_connection(socket: TcpStream) -> Result<()> {
    println!("Accepted new connection: {:?}", socket);
    let mut client_handler = ClientHandler::new(socket);
    // In a loop, read data from the socket and write the data back.
    loop {
        let value = client_handler.read_value().await?;
        dbg!(value);
    }
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
