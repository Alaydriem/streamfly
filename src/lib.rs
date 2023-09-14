use anyhow::Result;
use async_trait::async_trait;

mod client;
mod io;
mod msg;
mod server;
mod stream;

pub use crate::client::new_client;
pub use crate::server::serve;
pub use crate::stream::{Reader, Writer};

#[async_trait]
pub trait Client: Send + Sync {
    async fn open_stream(&mut self, channel: &str) -> Result<(String, Writer)>;

    async fn subscribe(
        &mut self,
        channel: &str,
    ) -> Result<async_channel::Receiver<(String, Reader)>>;

    async fn close(&mut self) -> Result<()>;
}
