use crate::session_store::{self, SessionStore};
use serde::{Deserialize, Serialize, Value};
use std::{collections::HashMap, result, sync::Arc};
use time::{Duration, OffsetDateTime};
type Result<T> = result::Result<T, Error>;

type Data = HashMap<String, Value>;

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Eq, Hash, PartialEq)]
pub struct Id(pub i128);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Record {
    pub id: Id,
    pub data: Data,
    pub expiry_date: OffsetDateTime,
}

#[derive(Debug)]
struct Inner {
    // This will be `None` when:
    //
    // 1. We have not been provided a session cookie or have failed to parse it,
    // 2. The store has not found the session.
    //
    // Sync lock, see: https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html#which-kind-of-mutex-should-you-use
    session_id: parking_lot::Mutex<Option<Id>>,

    // A lazy representation of the session's value, hydrated on a just-in-time basis. A
    // `None` value indicates we have not tried to access it yet. After access, it will always
    // contain `Some(Record)`.
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

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    Store(#[from] session_store::Error),
}
