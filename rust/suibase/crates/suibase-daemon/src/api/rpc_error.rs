// Convert various error type to a RpcError.
//
// All errors are map into one of the jsonrpsee "CallError" (e.g. InvalidParams, Failed, Custom).
//
// RpcInputError map to CallError::InvalidParams.
// RpcServerError map to CallError::Failed.

use jsonrpsee::core::Error as RpcError;
use jsonrpsee::types::error::CallError;
use thiserror::Error;

impl From<RpcInputError> for RpcError {
    fn from(e: RpcInputError) -> Self {
        e.rpc_error()
    }
}

impl From<RpcServerError> for RpcError {
    fn from(e: RpcServerError) -> Self {
        e.rpc_error()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RpcInputError {
    #[error("params {0} has invalid value '{1}'")]
    InvalidParams(String, String),
}

#[derive(Debug, thiserror::Error)]
pub enum RpcServerError {
    #[error("params {0} has invalid value '{1}'")]
    InvalidParams(String, String),
}

impl RpcInputError {
    pub fn rpc_error(self) -> RpcError {
        jsonrpsee::core::Error::Call(CallError::InvalidParams(self.into()))
    }
}

impl RpcServerError {
    pub fn rpc_error(self) -> RpcError {
        jsonrpsee::core::Error::Call(CallError::Failed(self.into()))
    }
}
