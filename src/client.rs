use std::{net::SocketAddr, path::Path};

use anyhow::Result;
use async_trait::async_trait;
use futures::{AsyncReadExt, AsyncWriteExt};
use s2n_quic::{
    connection::StreamAcceptor,
    provider::datagram::default::{Endpoint, Sender},
};
use serde::{Deserialize, Serialize};

use crate::{Client, Reader, Writer};

struct QuicClient {
    handle: s2n_quic::connection::Handle,
    rx: async_channel::Receiver<(String, Reader)>,
}

#[derive(Serialize, Deserialize)]
struct OpenStreamMsg {
    topic: String,
}

#[derive(Serialize, Deserialize)]
struct SubscribeMsg {
    topic: String,
}

#[async_trait]
impl Client for QuicClient {
    async fn open_stream(&mut self, topic: &str) -> Result<Writer> {
        let mut stream = self.handle.open_send_stream().await?;
        stream.write_all(&topic.as_bytes()).await?;
        Ok(Box::pin(stream))
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        self.handle.datagram_mut(|sender: &mut Sender| {
            sender.send_datagram(topic.to_owned().into()).unwrap();
            anyhow::Ok(())
        })??;
        Ok(())
    }

    async fn receive_stream(&mut self) -> Result<(String, Reader)> {
        let (topic, stream) = self.rx.recv().await?;
        Ok((topic, stream))
    }
}

async fn run_accept_streams(
    mut acceptor: StreamAcceptor,
    tx: async_channel::Sender<(String, Reader)>,
) -> Result<()> {
    loop {
        if let Some(mut stream) = acceptor.accept_receive_stream().await? {
            let mut buf = [0u8; 4];
            stream.read_exact(&mut buf).await?;
            let topic = String::from_utf8(buf.to_vec())?;
            let reader: Reader = Box::pin(stream);
            tx.send((topic, reader)).await?;
        }
    }
}

pub async fn connect(
    server_addr: SocketAddr,
    server_name: &str,
    cert: &Path,
) -> Result<Box<dyn Client>> {
    let datagram_provider = Endpoint::builder()
        .with_send_capacity(200)?
        .with_recv_capacity(200)?
        .build()
        .unwrap();

    let c: s2n_quic::Client = s2n_quic::Client::builder()
        .with_tls(cert)?
        .with_io("0.0.0.0:0")?
        .with_datagram(datagram_provider)?
        .start()
        .unwrap();

    let mut conn = c
        .connect(s2n_quic::client::Connect::new(server_addr).with_server_name(server_name))
        .await?;
    conn.keep_alive(true)?;

    let (handle, acceptor) = conn.split();
    let (tx, rx) = async_channel::unbounded();
    tokio::spawn(async move {
        run_accept_streams(acceptor, tx).await.unwrap();
    });

    Ok(Box::new(QuicClient { handle, rx }))
}
