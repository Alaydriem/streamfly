use std::path::Path;

use anyhow::Result;
use streamfly::serve;

#[tokio::main]
async fn main() -> Result<()> {
    serve(
        "0.0.0.0:1318",
        Path::new("./certs/cert.pem"),
        Path::new("./certs/key.pem"),
    )
    .await?;
    Ok(())
}
