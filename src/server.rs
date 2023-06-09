use std::{path::Path, sync::Arc, task::Poll};

use anyhow::Result;
use futures::{future::poll_fn, io, lock::Mutex};
use s2n_quic::{
    connection::{Handle, StreamAcceptor},
    provider::datagram::default::{Endpoint, Receiver},
};

use crate::{
    io::{read_packet, write_packet},
    msg::{OpenStreamMsg, SubscribeMsg},
    Reader, Writer,
};

pub async fn serve(addr: &str, cert: &Path, key: &Path) -> Result<()> {
    let datagram_provider = Endpoint::builder()
        .with_send_capacity(200)?
        .with_recv_capacity(200)?
        .build()
        .unwrap();

    let mut s = s2n_quic::Server::builder()
        .with_tls((cert, key))?
        .with_io(addr)?
        .with_datagram(datagram_provider)?
        .start()
        .unwrap();

    let all_handles = Arc::new(Mutex::new(Vec::default()));

    while let Some(conn) = s.accept().await {
        let all_handles_cloned = all_handles.clone();
        let (handle, acceptor) = conn.split();
        tokio::spawn(process_conn(all_handles_cloned, handle, acceptor));
    }

    Ok(())
}

async fn process_conn(
    all_handles: Arc<Mutex<Vec<(String, Handle)>>>,
    handle: Handle,
    mut acceptor: StreamAcceptor,
) -> Result<()> {
    loop {
        let all_handles_1 = all_handles.clone();
        let all_handles_2 = all_handles.clone();
        let handle_1 = handle.clone();

        tokio::select! {
            res = poll_fn(|cx| {
                match handle.datagram_mut(|recv: &mut Receiver| recv.poll_recv_datagram(cx)) {
                    Ok(poll_value) => poll_value.map(Ok),
                    Err(query_err) => Poll::Ready(Err(query_err)),
                }
            }) => {
                let msg = serde_json::from_slice::<SubscribeMsg>(&res?.unwrap().to_vec())?;
                let mut all_handles = all_handles_1.lock().await;
                all_handles.push((msg.topic, handle_1));
            }


            res = acceptor.accept_receive_stream() => {
                if let Some(stream) = res? {
                    tokio::spawn(process_stream(all_handles_2, Box::pin(stream)));
                }
            }
        }
    }
}

async fn process_stream(
    all_handles: Arc<Mutex<Vec<(String, Handle)>>>,
    mut stream: Reader,
) -> Result<()> {
    let msg = read_packet::<OpenStreamMsg>(&mut stream).await?;
    let mut all_handles = all_handles.lock().await;
    for (topic, handle) in all_handles.as_mut_slice() {
        if topic == &msg.topic {
            let mut w: Writer = Box::pin(handle.open_send_stream().await?);
            write_packet(&mut w, msg).await?;
            io::copy(stream, &mut w).await?;
            break;
        }
    }
    Ok(())
}
