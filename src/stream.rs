use std::pin::Pin;

use s2n_quic::stream::{ReceiveStream, SendStream};

pub type Reader = Pin<Box<ReceiveStream>>;
pub type Writer = Pin<Box<SendStream>>;

pub(crate) fn new_reader(s: ReceiveStream) -> Reader {
    Box::pin(s)
}

pub(crate) fn new_writer(s: SendStream) -> Writer {
    Box::pin(s)
}
