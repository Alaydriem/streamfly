use std::path::Path;

use anyhow::Result;
use streamfly::new_client;

const CHANNEL: &str = "demo-streamfly";

#[tokio::main]
async fn main() -> Result<()> {
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
            loop {
                match reader.receive().await? {
                    Some(buf) => {
                        println!("[{}]: {}", stream_id, String::from_utf8(buf.into())?);
                    }
                    None => {
                        println!("[{}]: EOF", stream_id);
                        break;
                    }
                }
            }
            anyhow::Ok(())
        });
    }
}
