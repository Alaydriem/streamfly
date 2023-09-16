# StreamFly

StreamFly aims to be a stream-oriented Pub/Sub framework.

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
    let (_, mut reader) = rx.recv().await?;

    tokio::spawn(async move { tokio::io::copy(&mut reader, &mut tokio::io::stdout()).await });
}
```

- publish a stream, and then write data to the stream

```rust
let (stream_id, mut writer) = client.open_stream(CHANNEL).await?;

writer.write_all(b"Hello, Streamfly!").await?;
```

## Build

- build streamfly cli command

```sh
RUSTFLAGS="--cfg s2n_quic_unstable" cargo build
```

- build examples

```sh
RUSTFLAGS="--cfg s2n_quic_unstable" cargo build --examples
```

## Run the demo

- start the streamfly server

```sh
RUST_LOG=debug ./target/debug/streamfly serve
```

- start a receiver

```sh
RUST_LOG=debug ./target/debug/examples/sub
```

- start a sender

```sh
RUST_LOG=debug ./target/debug/examples/pub
```

- you can start another receiver

```sh
RUST_LOG=debug ./target/debug/examples/sub
```
