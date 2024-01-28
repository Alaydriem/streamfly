use std::path::Path;

use anyhow::Result;
use streamfly::{ certificate::MtlsProvider, new_client };

const CHANNEL: &str = "demo-streamfly";

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let ca_cert = Path::new("./certs/ca.crt");
    let cert = Path::new("./certs/test.crt");
    let key = Path::new("./certs/test.key");

    let provider = MtlsProvider::new(ca_cert, cert, key).await?;

    let mut client = new_client("127.0.0.1:1318".parse()?, "localhost", provider).await?;

    let rx = client.subscribe(CHANNEL).await?;

    loop {
        let (_, mut reader) = rx.recv().await?;

        tokio::spawn(async move { tokio::io::copy(&mut reader, &mut tokio::io::stdout()).await });
    }
}
