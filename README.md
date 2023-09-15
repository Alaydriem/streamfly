# streamfly

Streamfly aims to be a stream-oriented Pub/Sub framework.

## Quickstart

- create a streamfly client

```rust
let mut client = new_client(
    "127.0.0.1:1318".parse()?,
    "localhost",
    Path::new("./certs/cert.pem"),
)
.await?;
```

- subscribe streams, and then receive data

```rust
let rx = client.subscribe(CHANNEL).await?;

loop {
    let (stream_id, mut reader) = rx.recv().await?;
    println!("accept new stream: {}", stream_id);

    tokio::spawn(async move {
        loop {
            match reader.receive().await? {
                Some(buf) => {
                    println!("[{}]: {}", stream_id, String::from_utf8(buf.into())?);
                }
                None => {
                    println!("[{}]: EOF", stream_id);
                    break;
                }
            }
        }
        anyhow::Ok(())
    });
}
```

- publish a stream, and then write data to the stream

```rust
let (stream_id, mut writer) = client.open_stream(CHANNEL).await?;
println!("publish new stream: {}", stream_id);

for i in 0..10 {
    let msg = format!("Hello, Streamfly [{}]!", i);
    println!("[{}]: {}", stream_id, msg);
    writer.write_all(msg.as_bytes()).await?;
    time::sleep(Duration::from_secs(1)).await;
}
```

## Build

- build streamfly cli command

```sh
cargo build
```

- build examples

```sh
cargo build --examples
```

## Run the demo

- start streamfly server

```sh
RUST_LOG=debug ./target/debug/streamfly serve
```

- start the subscriber

```sh
RUST_LOG=debug ./target/debug/examples/sub
```

- start the publisher

```sh
RUST_LOG=debug ./target/debug/examples/pub
```
