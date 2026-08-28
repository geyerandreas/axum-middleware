use crate::{AuthnBackend, session_core};
use std::fmt::Debug;

#[derive(thiserror::Error)]
pub enum Error<Backend: AuthnBackend> {
    #[error(transparent)]
    Session(session_core::Error),

    #[error(transparent)]
    Backend(Backend::Error),
}

impl<Backend: AuthnBackend> Debug for Error<Backend> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Session(e) => write!(f, "{e:?}")?,
            Error::Backend(e) => write!(f, "{e:?}")?,
        }
        Ok(())
    }
}

impl<Backend: AuthnBackend> From<session_core::Error> for Error<Backend> {
    fn from(value: session_core::Error) -> Self {
        Self::Session(value)
    }
}
