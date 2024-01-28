# StreamFly

StreamFly aims to be a stream-oriented Pub/Sub framework.

## Quickstart

Cargo.toml

```toml
[dependencies]
streamfly = "0.1"
```

#### Create a Client

```rust
let ca_cert = Path::new("./certs/ca.crt");
let cert = Path::new("./certs/test.crt");
let key = Path::new("./certs/test.key");

let provider = MtlsProvider::new(ca_cert, cert, key).await?;

let mut client = new_client("127.0.0.1:1318".parse()?, "localhost", provider).await?;
```

#### Subscribe

```rust
let rx = client.subscribe(CHANNEL).await?;

loop {
    let (_, mut reader) = rx.recv().await?;

    tokio::spawn(async move { tokio::io::copy(&mut reader, &mut tokio::io::stdout()).await });
}
```

#### Publish

```rust
let (stream_id, mut writer) = client.open_stream(CHANNEL).await?;

writer.write_all(b"Hello, Streamfly!").await?;
```

### Examples

```
# Start the server
cargo run --example server

# Start a publisher
cargo run --example pub
```

You can then spawn as many subscribers as you want

```
cargo run --example sub
```
