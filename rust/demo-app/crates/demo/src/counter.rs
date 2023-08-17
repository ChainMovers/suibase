// Send a transaction to call the "increment" Move function of a Counter object.
//
// type 'cargo run count' to send one transaction. After incrementing on the network,
// the Move program will emit an event.
//
// type 'cargo run events' in another terminal to see the event.
//
use std::path::PathBuf;

use shared_crypto::intent::Intent;
use sui_json_rpc_types::SuiTransactionBlockResponseOptions;
use sui_keys::keystore::{AccountKeystore, FileBasedKeystore, Keystore};
use sui_sdk::SuiClientBuilder;
use suibase::Helper;

use sui_sdk::json::SuiJsonValue;
use sui_types::quorum_driver_types::ExecuteTransactionRequestType;
use sui_types::transaction::Transaction;

use anyhow::ensure;

pub async fn count() -> Result<(), anyhow::Error> {
    // Use suibase to get information from the last published
    // packaged from this development machine. That includes:
    //    - the ObjectID of the Demo::Counter package published.
    //    - the ObjectID of the Counter object instantiated at publication.
    //
    // Also use suibase to help get what is needed to interact with the
    // network targeted for the demo:
    //    - Path to the keystore.
    //    - URLs to reach the network.
    //    - A client address

    // Initialize the suibase helper.
    let suibase = Helper::new();
    suibase.select_workdir("active")?;
    println!("Using suibase workdir [{}]", suibase.workdir()?);

    // Get information from the last publication.
    let package_id = suibase.package_object_id("demo")?;

    // Get the single Counter object that was created when the "demo" package was published.
    //
    // "demo::Counter::Counter" is for "package::Module::Type" defined in 'counter.move'.
    let object_ids = suibase.published_new_object_ids("demo::Counter::Counter")?;
    ensure!(
        object_ids.len() == 1,
        format!(
            "One counter object expected, but {} found instead.",
            object_ids.len()
        )
    );
    let counter_id = object_ids[0];
    // println!("demo::Counter ObjectID is: {}", counter_id);

    // Use the active client address (check the docs for useful alternatives for tests).
    let client_address = suibase.client_sui_address("active")?;

    // Get the keystore using the location given by suibase.
    let keystore_pathname = suibase.keystore_pathname()?;
    let keystore_pathbuf = PathBuf::from(keystore_pathname);
    let keystore = Keystore::File(FileBasedKeystore::new(&keystore_pathbuf)?);

    // Create a Sui client.
    let rpc_url = suibase.rpc_url()?;
    println!("Connecting to Sui network at [{}]", rpc_url);
    let sui_client = SuiClientBuilder::default().build(rpc_url).await?;

    // Send the transaction.
    let call_args = vec![SuiJsonValue::from_object_id(counter_id)];

    let move_call = sui_client
        .transaction_builder()
        .move_call(
            client_address,
            package_id,
            "Counter",
            "increment",
            vec![],
            call_args,
            None, // The node will pick a gas object belong to the signer if not provided.
            2000000,
        )
        .await?;

    let signature = keystore.sign_secure(&client_address, &move_call, Intent::sui_transaction())?;

    let tx = Transaction::from_data(move_call, Intent::sui_transaction(), vec![signature]);
    let response = sui_client
        .quorum_driver_api()
        .execute_transaction_block(
            tx,
            SuiTransactionBlockResponseOptions::new().with_effects(),
            Some(ExecuteTransactionRequestType::WaitForLocalExecution),
        )
        .await?;

    if response.errors.is_empty() {
        println!("Success. Transaction sent Digest {}", response.digest);
        println!("The Counter::increment function should run on the network and emit an");
        println!("event from package {}", package_id);
    } else {
        println!("Transaction failed. Errors:");
        for error in response.errors {
            println!("  {}", error);
        }
    }

    Ok(())
}
