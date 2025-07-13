# Slot

An extensible webserver.

The intent of this project is to enable incremental additions to a personal website without needing to rebuild existing code or restart processes.

The implementation works like this:
- A master server (the "Slot server") binds to a public facing port to handle all web traffic
- There can be any number of modules. Each functions as a standalone webserver but implements the Slot protocol to register with the Slot server. The modules run on the same machine as the Slot server
- When the Slot server receives an HTTP request, it forwards the request to a module, determined by the first segment of the URL endpoint

So for example, if the Slot server recieves a request for "/myproject/index.html", it will make an HTTP request for "/myproject/index.html" to the module it knows as "myproject".

## Usage

### Running the server example

```rust
cargo run -- --log "DEBUG" --web-bind "127.0.0.1:8000" --slot-bind 7568
```

### Implementing a module example

This crate comes with a Slot client implementation that makes implementing modules very straightforward

Include the "slot" crate in your Cargo.toml
```toml
[dependencies]
slot = { path = "../path/to/slot" }
```

Then simply invoke `slot_client::client_impl::run_client` to start the client on a separate thread. It will automatically handle errors and reconnect if the server goes down.
```rust
// Set up Slot client
let slot_port = 7568;
let module_name =
    slot_client::protocol::ValidName::from_str("mymodule")
        .expect("The constant module name is valid");
let my_http_addr = SocketAddr::from_str("127.0.0.1:8001").unwrap();

slot_client::client_impl::run_client(
    slot_port,
    module_name,
    my_http_addr.port(),
);

// Set up webserver
// Note: Only routes that begin with "/mymodule/" will be exposed
let routes = Router::new().route("/mymodule/index", get(test_route));

let listener = tokio::net::TcpListener::bind(my_http_addr).await.unwrap();
axum::serve(listener, routes).await.unwrap();
```

For the above example to work, the server's module listener should be at "127.0.0.1:7568". After setup is complete, you should be able to access the route "/mymodule/index" from both the module at "127.0.0.1:8001" and from the Slot server at whatever address it is bound to for HTTP requests.

For a more concrete example, see [bxyz-meta](https://github.com/blacepos/bxyz-meta)

## Limitations

Only the localhost interface is supported currently for the server module listener. Thus, only the port can be specified when running the server.

It is assumed that localhost is entirely inaccessible to even unprivileged users. Any process that can use localhost can register with the Slot server.

HTTPS logistics have not been implemented yet, so neither the Slot server nor any modules can use it.