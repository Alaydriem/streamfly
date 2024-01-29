use std::{ path::Path, time::Duration };

use anyhow::Result;
use streamfly::{ certificate::MtlsProvider, new_client };
use tokio::{ io::AsyncWriteExt, time };

const CHANNEL: &str = "demo-streamfly";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let ca_cert = Path::new("./certs/ca.crt");
    let cert = Path::new("./certs/test.crt");
    let key = Path::new("./certs/test.key");

    let provider = MtlsProvider::new(ca_cert, cert, key).await?;

    let mut client = new_client("127.0.0.1:1318".parse()?, "localhost", provider).await?;

    let (stream_id, mut writer) = client.open_stream(CHANNEL).await?;
    println!("publish new stream: {}", stream_id);

    for i in 1.. {
        let msg = format!("[{}] [{}] Hello, Streamfly!", stream_id, i);
        writer.write_all(msg.as_bytes()).await?;
        time::sleep(Duration::from_millis(1)).await;
    }
    writer.close().await?;
    client.close().await?;

    time::sleep(Duration::from_secs(1)).await;

    Ok(())
}
