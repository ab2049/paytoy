use serde::Deserialize;

/// The input client id
#[derive(Clone, Copy, Debug, Deserialize, Hash, Eq, Ord, PartialOrd, PartialEq)]
pub struct ClientId(pub u16);

impl ClientId {
    pub fn id(&self) -> u16 {
        self.0
    }
}

/// The input transaction id
#[derive(Clone, Copy, Debug, Deserialize, Hash, Eq, PartialEq)]
pub struct TxId(pub u32);

impl TxId {
    pub fn id(&self) -> u32 {
        self.0
    }
}
