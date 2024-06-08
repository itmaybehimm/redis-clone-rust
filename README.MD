Here's a `README.md` file that provides instructions on how to run the project, access the client using `redis-cli`, and explains the supported commands and concurrency control using `RwLock`.

````markdown
# Redis Rust Server

This project implements a simple Redis-like server in Rust, supporting basic commands such as `PING`, `ECHO`, `SET`, and `GET`. It also uses `RwLock` for concurrency control to ensure thread-safe access to shared data.

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (Make sure you have Rust installed)
- [Redis CLI](https://redis.io/topics/rediscli) (for testing the server)

### Running the Server

1. Clone the repository:
   ```sh
   git clone https://github.com/your-username/redis-rust-server.git
   cd redis-rust-server
   ```
````

2. Run the server using Cargo:
   ```sh
   cargo run
   ```

### Using Redis CLI

To interact with the server, you can use `redis-cli`. Open another terminal and use the following commands:

- **PING**: Test the connection to the server.

  ```sh
  redis-cli -p 6379 ping
  ```

- **ECHO**: Echo back the message you send.

  ```sh
  redis-cli -p 6379 echo "hello world"
  ```

- **SET**: Set a key-value pair in the server.

  ```sh
  redis-cli -p 6379 set mykey "myvalue"
  ```

- **GET**: Get the value associated with a key.
  ```sh
  redis-cli -p 6379 get mykey
  ```

### Concurrency Control

This project uses `RwLock` for concurrency control. `RwLock` allows multiple readers or a single writer at any point in time, ensuring thread-safe access to the shared `HashMap` that stores the key-value pairs.

### Example Session

Here is an example session using `redis-cli`:

```sh
$ redis-cli -p 6379 ping
PONG

$ redis-cli -p 6379 echo "hello world"
"hello world"

$ redis-cli -p 6379 set mykey "myvalue"
"OK"

$ redis-cli -p 6379 get mykey
"myvalue"
```

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Acknowledgements

- [Tokio](https://tokio.rs) for asynchronous runtime.
- [Redis](https://redis.io) for inspiration.

```

This `README.md` provides a comprehensive guide to running the server, using `redis-cli` for testing, and explains the concurrency control mechanism used in the project.
```
