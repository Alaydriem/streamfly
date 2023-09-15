use std::path::Path;

use anyhow::Result;
use streamfly::new_client;

const CHANNEL: &str = "demo-streamfly";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let mut client = new_client(
        "127.0.0.1:1318".parse()?,
        "localhost",
        Path::new("./certs/cert.pem"),
    )
    .await?;

    let rx = client.subscribe(CHANNEL).await?;

    loop {
        let (_, mut reader) = rx.recv().await?;

        tokio::spawn(async move { tokio::io::copy(&mut reader, &mut tokio::io::stdout()).await });
    }
}
