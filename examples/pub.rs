use std::{path::Path, time::Duration};

use anyhow::Result;
use futures::AsyncWriteExt;
use streamfly::new_client;
use tokio::time;

const CHANNEL: &str = "demo-streamfly";

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = new_client(
        "127.0.0.1:1318".parse()?,
        "localhost",
        Path::new("./certs/cert.pem"),
    )
    .await?;

    let mut writer = client.open_stream(CHANNEL).await?;

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
