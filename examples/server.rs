use std::path::Path;

use anyhow::Result;
use bytes::Bytes;
use futures::{ future::{ self, BoxFuture }, FutureExt };
use streamfly::{ certificate::MtlsProvider, serve };

#[tokio::main]
async fn main() -> Result<()> {
    let ca_cert = Path::new("./certs/ca.crt");
    let cert = Path::new("./certs/ca.crt");
    let key = Path::new("./certs/ca.key");

    let provider = MtlsProvider::new(ca_cert, cert, key).await?;

    match serve("127.0.0.1:1318", provider, mutator).await {
        Ok(listener) => {
            _ = listener.await;
        }
        Err(e) => {
            println!("{}", e.to_string());
        }
    }

    Ok(())
}

fn mutator(data: &[u8]) -> BoxFuture<'static, Result<Bytes, ()>> {
    let s = String::from_utf8_lossy(data);
    let f = s.to_string() + "foo\n";
    future::ready(Ok(f.into())).boxed()
}
