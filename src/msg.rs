use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct OpenStreamMsg {
    pub(crate) topic: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SubscribeMsg {
    pub(crate) topic: String,
}
