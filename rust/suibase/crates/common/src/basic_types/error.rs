#[derive(Debug, thiserror::Error)]
#[allow(clippy::large_enum_variant)]
pub enum SuibaseError {
    #[error("Internal error: {0}")]
    InternalError(String),
}
