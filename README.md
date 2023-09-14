# streamfly

Streamfly aims to be a stream-oriented Pub/Sub framework.

Unlike traditional Pub/Sub systems, instead of transffering data
packets(messages), streamfly focuses on transffering streams, which means users
could publish or subscrbie a stream(Reader & Writer). Then developers can
manipulate these streams in their applications, just like the stdin & stdout.

## Build

- build streamfly cli command

```sh
cargo build
```

- build examples

```sh
cargo build --examples
```

## Run

- start streamfly server

```sh
RUST_LOG=debug ./target/debug/streamfly serve
```

- subscribe a stream

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

```sh
./target/debug/examples/sub
```

- publish a stream, and then write data

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

```sh
./target/debug/examples/pub
```
