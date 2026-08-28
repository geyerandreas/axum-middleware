use std::fmt::Debug;

use async_trait::async_trait;
use time::OffsetDateTime;

use crate::session::Record;

impl Record {
    fn new(expiry_date: OffsetDateTime) -> Self {
        Self {
            id: Id::default(),
            data: Data::default(),
            expiry_date,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Encoding failed with: {0}")]
    Encode(String),

    #[error("Decoding failed with: {0}")]
    Decode(String),

    #[error("{0}")]
    Backend(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[async_trait]
pub trait SessionStore: Debug + Send + Sync + 'static {
    async fn create(&self, session_record: &mut Record) -> Result<()> {
        default_create(self, session_record).await
    }

    async fn save(&self, session_record: &Record) -> Result<()>;

    async fn load(&self, session_id: &Id) -> Result<Option<Record>>;

    async fn delete(&self, session_id: &Id) -> Result<()>;
}

async fn default_create<S: SessionStore + ?Sized>(
    store: &S,
    session_record: &mut Record,
) -> Result<()> {
    tracing::warn!(
        "The default implementation of `SessionStore::create` is being used, which relies on \
         `SessionStore::save`. To properly handle potential ID collisions, it is recommended that \
         stores implement their own version of `SessionStore::create`."
    );
    store.save(session_record).await?;
    Ok(())
}
