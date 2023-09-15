use std::{path::Path, time::Duration};

use anyhow::Result;
use futures::AsyncWriteExt;
use streamfly::new_client;
use tokio::time;

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

    let (stream_id, mut writer) = client.open_stream(CHANNEL).await?;
    println!("publish new stream: {}", stream_id);

    for i in 0..15 {
        let msg = format!("Hello, Streamfly [{}]!", i);
        println!("[{}]: {}", stream_id, msg);
        writer.send(msg.into()).await?;
        time::sleep(Duration::from_secs(1)).await;
    }
    writer.close().await?;
    client.close().await?;

    time::sleep(Duration::from_secs(1)).await;

    Ok(())
}
