use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct OpenStreamMsg {
    pub(crate) topic: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct SubscribeMsg {
    pub(crate) topic: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct SubscribeAckMsg {
    pub(crate) ok: bool,
    pub(crate) reason: String,
}
