use anyhow::Result;

use crate::log_safe_err;
trait JsonRpcValidation {
    fn jsonrpc(&self) -> &str;
    fn id(&self) -> i64;
    fn error(&self) -> Option<&serde_json::Value>;

    /// Validate common JSON-RPC requirements
    fn validate_json_rpc(&self) -> Result<()> {
        if self.jsonrpc() != "2.0" {
            return Err(anyhow::anyhow!("Invalid JSON-RPC version"));
        }
        if self.id() != 1 {
            return Err(anyhow::anyhow!("Invalid JSON-RPC ID"));
        }
        if let Some(error) = self.error() {
            let err_msg = format!(
                "Error JSON received: {}",
                serde_json::to_string_pretty(error)
                    .unwrap_or_else(|_| String::from("<failed to serialize error>"))
            );
            return Err(anyhow::anyhow!(err_msg));
        }
        Ok(())
    }
}

/// Generic struct for JSON-RPC responses with any result type
#[derive(serde::Deserialize)]
struct JsonRpcResponse<T> {
    jsonrpc: String,
    id: i64,
    #[serde(default)]
    error: Option<serde_json::Value>,
    result: T,
}

// Implement the validation trait for the generic response
impl<T> JsonRpcValidation for JsonRpcResponse<T> {
    fn jsonrpc(&self) -> &str {
        &self.jsonrpc
    }

    fn id(&self) -> i64 {
        self.id
    }

    fn error(&self) -> Option<&serde_json::Value> {
        self.error.as_ref()
    }
}

// Helper function for parsing and validating JSON-RPC responses
pub async fn parse_json_rpc_response<T>(response: reqwest::Response) -> Result<T>
where
    T: for<'de> serde::Deserialize<'de>,
{
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Response error: {}", response.status()));
    }

    let text = response.text().await?;

    match serde_json::from_str::<JsonRpcResponse<T>>(&text) {
        Ok(response_json) => {
            if let Err(e) = response_json.validate_json_rpc() {
                // Log the raw response when validation fails
                log_safe_err!(format!("JSON-RPC validation error: {}", e));
                log_safe_err!(format!("Raw response: {}", text));
                return Err(e);
            }
            Ok(response_json.result)
        }
        Err(e) => {
            // Log the raw response when parsing fails
            log_safe_err!(format!("Error parsing JSON-RPC response: {}", e));
            log_safe_err!(format!("Raw response: {}", text));
            Err(anyhow::anyhow!("Failed to parse JSON response: {}", e))
        }
    }
}
