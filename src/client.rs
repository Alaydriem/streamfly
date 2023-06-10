use std::{net::SocketAddr, path::Path, task::Poll};

use anyhow::{bail, Result};
use async_trait::async_trait;
use futures::future::poll_fn;
use log::error;
use s2n_quic::{
    connection::StreamAcceptor,
    provider::datagram::default::{Endpoint, Receiver, Sender},
};

use crate::{
    io::{read_packet, write_packet},
    msg::{OpenStreamMsg, SubscribeAckMsg, SubscribeMsg},
    Client, Reader, Writer,
};

struct QuicClient {
    handle: s2n_quic::connection::Handle,
    rx: async_channel::Receiver<(String, Reader)>,
}

#[async_trait]
impl Client for QuicClient {
    async fn open_stream(&mut self, topic: &str) -> Result<Writer> {
        let stream = self.handle.open_send_stream().await?;
        let msg = OpenStreamMsg {
            topic: topic.to_owned(),
        };
        let mut w: Writer = Box::pin(stream);
        write_packet(&mut w, msg).await?;
        Ok(w)
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        let msg = SubscribeMsg {
            topic: topic.to_owned(),
        };
        let buf = serde_json::to_vec(&msg)?;
        self.handle.datagram_mut(|sender: &mut Sender| {
            if let Err(e) = sender.send_datagram(buf.into()) {
                bail!(e);
            }
            anyhow::Ok(())
        })??;

        match poll_fn(|cx| {
            match self
                .handle
                .datagram_mut(|recv: &mut Receiver| recv.poll_recv_datagram(cx))
            {
                Ok(value) => value.map(Ok),
                Err(err) => Poll::Ready(Err(err)),
            }
        })
        .await?
        {
            Ok(buf) => {
                let msg = serde_json::from_slice::<SubscribeAckMsg>(&buf)?;
                if !msg.ok {
                    bail!(msg.reason);
                }
            }
            Err(e) => {
                bail!(e);
            }
        }

        Ok(())
    }

    async fn receive_stream(&mut self) -> Result<(String, Reader)> {
        let (topic, stream) = self.rx.recv().await?;
        Ok((topic, stream))
    }

    async fn close(&mut self) -> Result<()> {
        self.handle.close(1u32.into());
        Ok(())
    }
}

async fn run_accept_streams(
    mut acceptor: StreamAcceptor,
    tx: async_channel::Sender<(String, Reader)>,
) -> Result<()> {
    while let Some(stream) = acceptor.accept_receive_stream().await? {
        let mut reader: Reader = Box::pin(stream);
        let msg = read_packet::<OpenStreamMsg>(&mut reader).await?;
        tx.send((msg.topic, reader)).await?;
    }
    Ok(())
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
        if let Err(e) = run_accept_streams(acceptor, tx).await {
            error!("{}", e);
        }
    });

    Ok(Box::new(QuicClient { handle, rx }))
}
