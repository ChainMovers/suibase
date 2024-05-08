// Convert various error type to a RpcError.
//
// All errors are map into one of the jsonrpsee "CallError" (e.g. InvalidParams, Failed, Custom).
//
// RpcInputError map to CallError::InvalidParams.
// RpcServerError map to CallError::Failed.

use jsonrpsee_types::ErrorObjectOwned as RpcError;

impl From<RpcInputError> for RpcError {
    fn from(e: RpcInputError) -> Self {
        e.rpc_error()
    }
}

impl From<RpcSuibaseError> for RpcError {
    fn from(e: RpcSuibaseError) -> Self {
        e.rpc_error()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RpcInputError {
    #[error("params {0} has invalid value '{1}'")]
    InvalidParams(String, String),
}

#[derive(Debug, thiserror::Error)]
pub enum RpcSuibaseError {
    #[error("internal error: {0}")]
    InternalError(String),
    #[error("file access error: {0}")]
    FileAccessError(String),
    #[error("outdated uuid")]
    OutdatedUUID(),
    #[error("Problem with suibase.yaml config: {0}")]
    InvalidConfig(String),
    #[error("Problem getting local Host: {0}")]
    LocalHostError(String),
    #[error("Remote Host does not exists: {0}")]
    RemoteHostDoesNotExists(String),
    #[error("Could not create connection. {0}")]
    ConnectionCreationFailed(String),
}

impl RpcInputError {
    pub fn rpc_error(self) -> RpcError {
        let message = format!("{}", self);
        jsonrpsee_types::ErrorObject::owned(
            jsonrpsee_types::error::ErrorCode::InvalidParams.code(),
            message,
            None::<()>,
        )
    }
}

impl RpcSuibaseError {
    pub fn rpc_error(self) -> RpcError {
        let message = format!("{}", self);
        jsonrpsee_types::ErrorObject::owned(
            jsonrpsee_types::error::ErrorCode::InternalError.code(),
            message,
            None::<()>,
        )
    }
}
