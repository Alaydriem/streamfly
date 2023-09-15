use std::{path::Path, time::Duration};

use anyhow::Result;
use streamfly::new_client;
use tokio::{io::AsyncWriteExt, time};

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

    for i in 1.. {
        let msg = format!("[{}] [{}] Hello, Streamfly!\n", stream_id, i);
        print!("{}", msg);
        writer.write_all(msg.as_bytes()).await?;
        time::sleep(Duration::from_secs(1)).await;
    }
    writer.close().await?;
    client.close().await?;

    time::sleep(Duration::from_secs(1)).await;

    Ok(())
}
