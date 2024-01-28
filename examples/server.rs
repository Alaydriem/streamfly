use std::path::Path;

use anyhow::Result;
use streamfly::{ certificate::MtlsProvider, serve };

#[tokio::main]
async fn main() -> Result<()> {
    let ca_cert = Path::new("./certs/ca.crt");
    let cert = Path::new("./certs/ca.crt");
    let key = Path::new("./certs/ca.key");

    let provider = MtlsProvider::new(ca_cert, cert, key).await?;

    _ = serve("127.0.0.1:1318", provider).await;

    Ok(())
}
