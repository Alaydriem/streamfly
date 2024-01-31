# StreamFly

StreamFly aims to be a stream-oriented Pub/Sub framework.

## Quickstart

Cargo.toml

```toml
[dependencies]
streamfly = { git = "https://github.com/alaydriem/streamfly" }
```

#### Create a Server

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let ca_cert = Path::new("./certs/ca.crt");
    let cert = Path::new("./certs/ca.crt");
    let key = Path::new("./certs/ca.key");

    let provider = MtlsProvider::new(ca_cert, cert, key).await?;

    match serve("127.0.0.1:1318", provider, mutator).await {
        Ok(listener) => {
            _ = listener.await;
        }
        Err(e) => {
            println!("{}", e.to_string());
        }
    }

    Ok(())
}

fn mutator(data: &[u8]) -> BoxFuture<'static, Result<Bytes, ()>> {
    let s = String::from_utf8_lossy(data);
    future::ready(Ok(s.into())).boxed()
}

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
