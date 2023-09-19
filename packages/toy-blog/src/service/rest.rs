pub mod api;
pub mod cors;
pub mod repository;
pub mod auth;
pub mod exposed_representation_format;

use inner_no_leak::ComposeInternalError;

mod inner_no_leak {
    use std::error::Error;
    use thiserror::Error;

    pub type ComposeInternalError<T> = Result<T, UnhandledError>;

    #[derive(Error, Debug)]
    #[error("Internal error: {_0}")]
    pub struct UnhandledError(pub Box<dyn Error>);

    impl UnhandledError {
        pub fn new<E: Error + 'static>(error: E) -> Self {
            Self(Box::new(error) as _)
        }
    }
}
