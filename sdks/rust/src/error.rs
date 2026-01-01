use std::sync::Arc;

use thiserror::Error;

#[derive(Error, Debug, Clone)]
#[non_exhaustive]
pub enum Error {
    #[error("Connection is already disconnected or has terminated normally")]
    Disconnected,

    #[error("Failed to connect: {source}")]
    FailedToConnect {
        #[source]
        source: InternalError,
    },

    #[error("Host returned error when processing subscription query: {error}")]
    SubscriptionError { error: String },

    #[error("Subscription has already ended")]
    AlreadyEnded,

    #[error("Unsubscribe already called on subscription")]
    AlreadyUnsubscribed,

    #[error(transparent)]
    Internal(#[from] InternalError),
}

#[derive(Debug, Clone)]
pub struct InternalError {
    message: String,
    cause: Option<Arc<dyn std::error::Error + Send + Sync>>,
}

impl InternalError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            cause: None,
        }
    }

    #[doc(hidden)]
    /// Called by codegen. Not part of this library's stable API.
    pub fn with_cause(self, cause: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self {
            cause: Some(Arc::new(cause)),
            ..self
        }
    }

    #[doc(hidden)]
    /// Called by codegen. Not part of this library's stable API.
    pub fn failed_parse(ty: &'static str, container: &'static str) -> Self {
        Self::new(format!(
            "Failed to parse {ty} from {container}.

This is often caused by outdated bindings; try re-running `spacetime generate`."
        ))
    }

    #[doc(hidden)]
    /// Called by codegen. Not part of this library's stable API.
    pub fn unknown_name(category: &'static str, name: impl std::fmt::Display, container: &'static str) -> Self {
        Self::new(format!(
            "Unknown {category} {name} in {container}

This is often caused by outdated bindings; try re-running `spacetime generate`."
        ))
    }
}

impl std::fmt::Display for InternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.message)?;
        if let Some(cause) = &self.cause {
            write!(f, ": {cause}")?;
        }
        Ok(())
    }
}

impl std::error::Error for InternalError {
    fn description(&self) -> &str {
        &self.message
    }

    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.cause
            .as_deref()
            .map(|cause| cause as &(dyn std::error::Error + 'static))
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn internal_error_without_cause_has_no_source() {
        let error = InternalError::new("outer context");
        assert!(error.source().is_none());
        assert!(error.clone().source().is_none());
        assert_eq!(error.to_string(), "outer context");
    }

    #[test]
    fn internal_error_source_is_the_stored_error_not_its_arc() {
        let error = InternalError::new("outer context").with_cause(Error::Disconnected);
        let source = error.source().unwrap().downcast_ref::<Error>().unwrap();
        assert!(matches!(source, Error::Disconnected));

        let cloned = error.clone();
        let cloned_source = cloned.source().unwrap().downcast_ref::<Error>().unwrap();
        assert!(std::ptr::eq(source, cloned_source));
        assert_eq!(
            error.to_string(),
            "outer context: Connection is already disconnected or has terminated normally",
        );
    }

    #[test]
    fn internal_error_source_preserves_nested_typed_causes() {
        let inner = InternalError::new("inner context")
            .with_cause(std::io::Error::new(std::io::ErrorKind::TimedOut, "fixture timeout"));
        let error = InternalError::new("outer context").with_cause(inner);

        let inner = error.source().unwrap().downcast_ref::<InternalError>().unwrap();
        let cause = inner.source().unwrap().downcast_ref::<std::io::Error>().unwrap();
        assert_eq!(cause.kind(), std::io::ErrorKind::TimedOut);
        assert!(cause.source().is_none());
        assert_eq!(error.to_string(), "outer context: inner context: fixture timeout");
    }
}

