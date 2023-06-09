use std::pin::Pin;

use anyhow::Result;
use async_trait::async_trait;
use futures::{AsyncRead, AsyncWrite};

mod client;
mod io;
mod msg;
mod server;

pub type Reader = Pin<Box<dyn AsyncRead + Send + Sync>>;
pub type Writer = Pin<Box<dyn AsyncWrite + Send + Sync>>;

#[async_trait]
pub trait Client: Send + Sync {
    async fn open_stream(&mut self, topic: &str) -> Result<Writer>;

    async fn subscribe(&mut self, topic: &str) -> Result<()>;

    async fn receive_stream(&mut self) -> Result<(String, Reader)>;

    async fn close(&mut self) -> Result<()>;
}

pub use crate::client::connect;
pub use crate::server::serve;
