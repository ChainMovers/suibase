//use sui_sdk::types::{ObjectID, SuiAddress};
use anyhow;
//use std::backtrace::Backtrace;
use core::option::Option;
use sui_sdk::types::error::SuiError;
use thiserror;
use super::sui_sdk_wrapped::{SuiSDKParamsRPC, SuiSDKParamsTxn};

#[derive(Debug, thiserror::Error)]
#[allow(clippy::large_enum_variant)]
pub enum DTPError {
    #[error("DTP Must create a Localhost first")]
    DTPLocalhostDoesNotExists,

    #[error("DTP Localhost {localhost:?} already exists for client {client:?}")]
    DTPLocalhostAlreadyExists { localhost: String, client: String },

    #[error("DTP Localhost data missing. Network accessible?")]
    DTPLocalhostDataMissing,

    #[error("DTP Missing Server Admin Address")]
    DTPMissingServerAdminAddress,

    #[error("DTP Object ID not found")]
    DTPObjectIDNotFound,

    #[error("DTP Internal Error. Sui Client Should Exist.")]
    DTPMissingSuiClient,

    #[error("DTP Failed Move call {desc:?} into package {package_id:?} for client {client_address:?}. Info from sui_sdk-> {inner:?}")]
    DTPFailedMoveCall { desc: String, package_id: String, client_address: String, inner: String },

    #[error(
        "DTP Failed fetching object {object_type:?}::{object_id:?}. Info from sui_sdk-> {inner:?}"
    )]
    DTPFailedFetchObject {
        object_type: String,
        object_id: String,
        inner: String,
    },

    #[error("DTP Move BCS to Rust object mapping failed {object_type:?}::{object_id:?} raw_data={raw_data:?}")]
    DTPFailedConvertBCS {
        object_type: String,
        object_id: String,
        raw_data: String,
    },

    #[error("DTP Support for more than one RPC node not yet implemented. Consider contributing.")]
    DTPMultipleRPCNotImplemented,

    #[error(
        "DTP Failed RPC get_objects_owned_by_address({client:?}). Info from sui_sdk-> {inner:?}"
    )]
    FailedRPCGetObjectsOwnedByClientAddress { client: String, inner: String },

    // Terminated. Will need to re-create/re-open.
    #[error("DTP Package ID not found")]
    PackageIDNotFound,

    #[error("DTP Object ID not found")]
    TestHelperObjectNotFound,

    #[error("DTP Not yet implemented. Need it? Ask for it on DTP Discord (Not Sui Discord).")]
    DTPNotImplemented,

    #[error("DTP Internal Error. Report to DTP developer please. Thanks.")]
    DTPInternalError,

    #[error("DTP inner SuiError {0:?}")]
    InnerSuiError(#[from] SuiError),

    #[error("DTP inner anyhow::Error {0:?}")]
    InnerAnyhowError(#[from] anyhow::Error),
}

pub struct MoreInfo {
    pub fix_caller_into_dtp_api: bool,
    pub internal_err_report_to_devs: bool,
}

// DTP API uses always anyhow::Error.
//
// Actionable info for the API user are obtain through
// functions provided here.
//
// This information is displayed by anyhow AND provided
// here for customized error handling.

pub fn get_more_info(err: anyhow::Error) -> Option<MoreInfo> {
    // Try to downcast to DTPError and match
    // the logic to build the MoreInfo.
    match err.downcast::<DTPError>() {
        Ok(dtp_err) => dtp_err.more_info(),
        Err(_anyhow_error) => None,
    }
}

impl DTPError {
    pub fn more_info(&self) -> Option<MoreInfo> {
        match self {
            DTPError::DTPObjectIDNotFound => Some(MoreInfo {
                fix_caller_into_dtp_api: false,
                internal_err_report_to_devs: false,
            }),
            DTPError::DTPLocalhostDoesNotExists => Some(MoreInfo {
                fix_caller_into_dtp_api: true,
                internal_err_report_to_devs: false,
            }),
            _ => None,
        }
    }
}
