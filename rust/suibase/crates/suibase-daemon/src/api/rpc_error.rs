// Convert various error type to a RpcError.
//
// All errors are map into one of the jsonrpsee "CallError" (e.g. InvalidParams, Failed, Custom).
//
// Define RpcInputError that is to be map to CallError::InvalidParams.
//
use crate::basic_types::SuibaseError;
use jsonrpsee::core::Error as RpcError;
use jsonrpsee::types::error::CallError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    InternalError(#[from] anyhow::Error),

    #[error(transparent)]
    RPCServerError(#[from] jsonrpsee::core::Error),

    #[error(transparent)]
    SuibaseError(#[from] SuibaseError),

    #[error(transparent)]
    RpcInputError(#[from] RpcInputError),
}

impl From<Error> for RpcError {
    fn from(e: Error) -> Self {
        e.to_rpc_error()
    }
}

impl Error {
    pub fn to_rpc_error(self) -> RpcError {
        // Convert any error to one of the few CallError supported.
        match self {
            Error::RpcInputError(json_rpc_input_error) => {
                RpcError::Call(CallError::InvalidParams(json_rpc_input_error.into()))
            }
            _ => RpcError::Call(CallError::Failed(self.into())),
        }
    }
}

#[derive(Debug, Error)]
pub enum RpcInputError {
    #[error("Invalid workdir parameter {0}")]
    InvalidWorkdirParameter(String),
}
