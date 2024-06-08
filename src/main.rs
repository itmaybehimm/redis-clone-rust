use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use redis_rust::ClientRequest;

async fn handle_connection(mut socket: TcpStream) -> io::Result<()> {
    println!("Accepted new connection: {:?}", socket);
    let mut buf = [0; 1024];

    // In a loop, read data from the socket and write the data back.
    loop {
        //calulating size of buffer
        let size = match socket.read(&mut buf).await {
            // socket closed
            Ok(size) if size == 0 => return Ok(()),
            Ok(size) => size,
            Err(e) => {
                eprintln!("failed to read from socket; err = {:?}", e);
                return Err(e);
            }
        };

        let message = String::from_utf8_lossy(&buf[0..size]);

        let request = ClientRequest::from(&message);
        dbg!(&request);

        let response = if request.elements[0] == "PING" {
            "+PONG\r\n"
        } else {
            "-Error unknown command\r\n"
        };

        // Write the data back
        if let Err(e) = socket.write_all(response.as_bytes()).await {
            eprintln!("failed to write to socket; err = {:?}", e);
            return Err(e);
        }
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
