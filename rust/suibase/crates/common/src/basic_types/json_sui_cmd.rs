// Utilities to parse some sui commands JSON output

use anyhow::{anyhow, Result};

// Parse Mist and Frost JSON output
pub fn parse_sui_balances(json: &str) -> Result<(u64, u64)> {
    // Parse the JSON response
    // Done by summing the "balance" fields of coinType ending with "sui::SUI" and "wal::WAL" respectively.
    //
    // Example of json output:
    //[
    //  [
    //    [
    //      {
    //        "decimals": 9,
    //        "name": "Sui",
    //        "symbol": "SUI",
    //        "description": "",
    //        "iconUrl": null,
    //        "id": "0x587c29de216efd4219573e08a1f6964d4fa7cb714518c2c8a0f29abfa264327d"
    //      },
    //      [
    //        {
    //          "coinType": "0x2::sui::SUI",
    //          "coinObjectId": "0xb3e0e5b281849495efe7ac41222a04d85beebd5a6fa84cb764d3f018652e7b2c",
    //          "version": "400613340",
    //          "digest": "G1PjSPrmAVTRT5gRDiCyAfBnj74vantcY9W9CFebG8fU",
    //          "balance": "69321181217",
    //          "previousTransaction": "5bqXtYLvsWd1caYmnNYGkprGgQzfJJoZr2tL4JX66ema"
    //        },
    //        {
    //          "coinType": "0x2::sui::SUI",
    //          "coinObjectId": "0x02660fe930a85c3259610cf87e4fe78f791885b2381ba0a5ce51e8cd6b84eac9",
    //          "version": "353196905",
    //          "digest": "9tqQR34qKK58aystEgVLAuibPkug8zBKDzE8kzpD4bjM",
    //          "balance": "934731232",
    //          "previousTransaction": "J855VdBTd1D7Fd8JjjBy9oKkC5QNxZN4ahhmBLa7dC1v"
    //        }
    //      ]
    //    ],
    //    [
    //      {
    //        "decimals": 9,
    //        "name": "WAL Token",
    //        "symbol": "WAL",
    //        "description": "The native token for the Walrus Protocol.",
    //        "iconUrl": "https://www.walrus.xyz/wal-icon.svg",
    //        "id": "0x27e59c4f7998e6f1b4567ad460439ca4bbe6b14f2d7ce2206d75519cefd9bf02"
    //      },
    //      [
    //        {
    //          "coinType": "0x8270feb7375eee355e64fdb69c50abb6b5f9393a722883c1cf45f8e26048810a::wal::WAL",
    //          "coinObjectId": "0xd0424f45984cd92a5246df59fb107a538bdc6e2eba4529b0a3a1bdf6d2bef5ca",
    //          "version": "400613338",
    //          "digest": "8ykwMRkP923quGY9uGbfEHMGwZqBg5RfLPa6tfVqzUwp",
    //          "balance": "433835000",
    //          "previousTransaction": "8qA3XqmWNNA26igkhMPK8CdXGvTHLHtAy7HnnH1rxVth"
    //        }...
    //      ]
    //    ]
    //  ],
    //  false
    //]

    let json_value: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| anyhow!("Failed to parse balance JSON: {} Output is {}", e, json))?;

    // Initialize balance accumulators
    let mut mist_balance: u64 = 0;
    let mut frost_balance: u64 = 0;

    // The first level is an array with the first element containing token groups
    if let Some(token_groups) = json_value.get(0).and_then(|v| v.as_array()) {
        // Iterate through each token group (SUI, WAL, etc)
        for token_group in token_groups {
            if let Some(group_array) = token_group.as_array() {
                if group_array.len() < 2 {
                    continue; // Skip if structure isn't as expected
                }

                // Second element is the array of coin objects
                if let Some(coins) = group_array.get(1).and_then(|v| v.as_array()) {
                    for coin in coins {
                        // Extract coin type and balance
                        let coin_type = coin
                            .get("coinType")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();

                        if let Some(balance_str) = coin.get("balance").and_then(|v| v.as_str()) {
                            // Parse balance as u64
                            if let Ok(balance) = balance_str.parse::<u64>() {
                                // Accumulate based on coin type
                                if coin_type.ends_with("sui::SUI") {
                                    mist_balance = mist_balance.saturating_add(balance);
                                } else if coin_type.ends_with("wal::WAL") {
                                    frost_balance = frost_balance.saturating_add(balance);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok((mist_balance, frost_balance))
}
