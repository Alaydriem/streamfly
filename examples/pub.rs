use std::{path::Path, time::Duration};

use anyhow::Result;
use futures::AsyncWriteExt;
use streamfly::{connect, Client};
use tokio::time;

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
    let mut writer = client.open_stream(TOPIC).await?;

    for i in 0..5 {
        let msg = format!("{}: Hello, Streamfly!\n", i);
        print!("{}", msg);
        writer.write_all(msg.as_bytes()).await?;
        time::sleep(Duration::from_secs(1)).await;
    }
    writer.close().await?;
    client.close().await?;

    Ok(())
}
