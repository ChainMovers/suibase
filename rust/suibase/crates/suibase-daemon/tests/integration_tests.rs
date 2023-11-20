// These integration tests assume:
//  - localnet is already installed

use log;
use serde_json::json;

fn init() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Info)
        .try_init();
}

async fn api_request(method: &str) -> serde_json::Value {
    let client = reqwest::Client::new();
    let request_url = "http://0.0.0.0:44399";
    let request_body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": ["localnet"]
    });

    let response = match client.post(request_url).json(&request_body).send().await {
        Ok(response) => response,
        Err(e) => {
            log::error!("api_request error: {:?}", e);
            assert!(false);
            return serde_json::Value::Null; // Dummy code, will never reach here with above assert.
        }
    };
    assert!(response.status().is_success());
    let response_body = response.text().await.unwrap();
    let value = serde_json::from_str(&response_body);
    assert!(value.is_ok());
    let value: serde_json::Value = value.unwrap();

    // Some sanity checks.
    let hdr_method = value["result"]["header"]["method"].as_str().unwrap();
    assert_eq!(hdr_method, method);
    let _ = value["result"]["header"]["methodUuid"].as_str().unwrap();
    let _ = value["result"]["header"]["dataUuid"].as_str().unwrap();
    value
}

#[tokio::test]
async fn test_sanity_api() {
    init();
    // Do a JSON-RPC 2.0 call of the method getStatus at http://0.0.0.0:44340
    let response = api_request("getStatus").await;

    // Example of valid response:
    // {"jsonrpc":"2.0","result":{
    // "header":{"method":"getStatus", "methodUuid":"1E7...61G","dataUuid":"065...2N0","key":"localnet"},
    // "status":"OK",
    // "services":[{"label":"localnet process","status":"OK","statusInfo":null,"helpInfo":null,"pid":null},
    // {"label":"faucet process","status":"OK","statusInfo":null,"helpInfo":null,"pid":null},
    // {"label":"proxy server","status":"OK","statusInfo":null,"helpInfo":null,"pid":null},
    // {"label":"multi-link RPC","status":"OK","statusInfo":null,"helpInfo":null,"pid":null}]
    // },"id":1}
    log::info!("response_body: {}", response);
    assert_eq!(response["result"]["status"].as_str().unwrap(), "OK");
}
