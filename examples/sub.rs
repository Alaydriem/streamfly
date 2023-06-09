use std::path::Path;

use anyhow::Result;
use futures::{io::BufReader, AsyncBufReadExt};
use streamfly::{connect, Client};

async fn new_client() -> Result<Box<dyn Client>> {
    Ok(connect(
        "127.0.0.1:1318".parse()?,
        "localhost",
        Path::new("./certs/cert.pem"),
    )
    .await?)
}

const TOPIC: &str = "abcd";

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = new_client().await?;
    client.subscribe(TOPIC).await?;

    let (_topic, stream) = client.receive_stream().await?;
    let mut buf_reader = BufReader::new(stream);

    loop {
        let mut line = String::new();
        let len = buf_reader.read_line(&mut line).await?;
        if len <= 0 {
            break;
        }
        print!("{}", line);
    }
    client.close().await?;

    Ok(())
}
