use crate::session_store::{self, SessionStore};
use base64::{DecodeError, Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fmt::{self, Display},
    result,
    str::FromStr,
    sync::{Arc, atomic::AtomicBool},
};
use time::{Duration, OffsetDateTime};
use tokio::sync::Mutex;
type Result<T> = result::Result<T, Error>;

type Data = HashMap<String, Value>;

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Eq, Hash, PartialEq)]
pub struct Id(pub i128);

impl Default for Id {
    fn default() -> Self {
        use rand::prelude::*;

        Self(rand::rng().random())
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut encoded = [0; 22];
        URL_SAFE_NO_PAD
            .encode_slice(self.0.to_le_bytes(), &mut encoded)
            .expect("Encoded ID must be exactly 22 bytes");
        let encoded = str::from_utf8(&encoded).expect("Encoded ID must be valid UTF-8");

        f.write_str(encoded)
    }
}

impl FromStr for Id {
    type Err = base64::DecodeSliceError;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        let mut decoded = [0; 16];
        let bytes_decoded = URL_SAFE_NO_PAD.decode_slice(s.as_bytes(), &mut decoded)?;
        if bytes_decoded != 16 {
            let err = DecodeError::InvalidLength(bytes_decoded);
            return Err(base64::DecodeSliceError::DecodeError(err));
        }

        Ok(Self(i128::from_le_bytes(decoded)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Record {
    pub id: Id,
    pub data: Data,
    pub expiry_date: OffsetDateTime,
}

impl Record {
    fn new(expiry_date: OffsetDateTime) -> Self {
        Self {
            id: Id::default(),
            data: Data::default(),
            expiry_date,
        }
    }
}

#[derive(Debug)]
struct Inner {
    session_id: parking_lot::Mutex<Option<Id>>,

    record: Mutex<Option<Record>>,

    // Sync lock, see: https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html#which-kind-of-mutex-should-you-use
    expiry: parking_lot::Mutex<Option<Expiry>>,

    is_modified: AtomicBool,
}

#[derive(Debug, Clone)]
pub struct Session {
    store: Arc<dyn SessionStore>,
    inner: Arc<Inner>,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Expiry {
    OnSessionEnd,
    OnInactivity(Duration),
    AtDateTime(OffsetDateTime),
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    Store(#[from] session_store::Error),
}
