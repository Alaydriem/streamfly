use anyhow::Result;
use async_trait::async_trait;

mod client;
mod io;
mod msg;
mod server;
mod stream;

pub use crate::client::connect;
pub use crate::server::serve;
pub use crate::stream::{Reader, Writer};

#[async_trait]
pub trait Client: Send + Sync {
    async fn open_stream(&mut self, topic: &str) -> Result<Writer>;

    async fn subscribe(&mut self, topic: &str) -> Result<()>;

    async fn receive_stream(&mut self) -> Result<(String, Reader)>;

    async fn close(&mut self) -> Result<()>;
}
