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
        let mut reader = rx.recv().await?;
        tokio::spawn(async move {
            loop {
                match reader.receive().await? {
                    Some(buf) => println!("recv: {}", String::from_utf8(buf.into())?),
                    None => {
                        println!("EOF");
                        break;
                    }
                }
            }
            anyhow::Ok(())
        });
    }
}
