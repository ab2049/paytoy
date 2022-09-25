use serde::Deserialize;

/// The input client id
#[derive(Clone, Copy, Debug, Deserialize, Hash, Eq, PartialEq)]
pub struct ClientId(pub u16);

/// The input transaction id
#[derive(Clone, Copy, Debug, Deserialize, Hash, Eq, PartialEq)]
pub struct TxId(pub u32);
