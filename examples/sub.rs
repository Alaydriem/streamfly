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
        let (stream_id, mut reader) = rx.recv().await?;
        println!("accept new stream: {}", stream_id);

        tokio::spawn(async move {
            while let Some(buf) = reader.receive().await? {
                println!("[{}]: {}", stream_id, String::from_utf8(buf.into())?);
            }
            println!("[{}]: EOF", stream_id);
            anyhow::Ok(())
        });
    }
}
