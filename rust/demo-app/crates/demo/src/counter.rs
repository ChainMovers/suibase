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

use sui_sdk::json::SuiJsonValue;
use sui_sdk::types::messages::Transaction;
use sui_types::messages::ExecuteTransactionRequestType;

use sui_base_helper::SuiBaseHelper;

use anyhow::ensure;

pub async fn count() -> Result<(), anyhow::Error> {
    // Use sui-base to get information from the last published
    // packaged from this development machine. That includes:
    //    - the ObjectID of the Demo::Counter package published.
    //    - the ObjectID of the Counter object instantiated at publication.
    //
    // Also use sui-base to help get what is needed to interact with the
    // network targeted for the demo:
    //    - Path to the keystore.
    //    - URLs to reach the network.
    //    - A client address

    // Initialize the sui-base helper.
    let mut suibase = SuiBaseHelper::new();
    suibase.select_workdir("localnet")?;

    // Get information from the last publication.
    let package_id = suibase.get_package_id("demo")?;

    // Get the single Counter object that was created when the "demo" package was published.
    //
    // "demo::Counter::Counter" is for "package::Module::Type" defined in 'counter.move'.
    let object_ids = suibase.get_published_new_objects("demo::Counter::Counter")?;
    ensure!(
        object_ids.len() == 1,
        format!(
            "One counter object expected, but {} found instead.",
            object_ids.len()
        )
    );
    let counter_id = object_ids[0];

    // Use a client address known to be always created by sui-base for localnet/devnet/tesnet.
    let client_address = suibase.get_client_address("sb-1-ed25519")?;

    // Get the keystore using the location given by sui-base.
    let keystore_pathname = suibase.get_keystore_pathname()?;
    let keystore_pathbuf = PathBuf::from(keystore_pathname);
    let keystore = Keystore::File(FileBasedKeystore::new(&keystore_pathbuf)?);

    // TODO Get URL from sui-base ( https://github.com/sui-base/sui-base/issues/6 )
    let sui_client = SuiClientBuilder::default()
        .build("http://0.0.0.0:9000")
        .await?;

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
            20000,
        )
        .await?;

    let signature = keystore.sign_secure(&client_address, &move_call, Intent::default())?;

    let _response = sui_client
        .quorum_driver()
        .execute_transaction_block(
            Transaction::from_data(move_call, Intent::default(), vec![signature]).verify()?,
            SuiTransactionBlockResponseOptions::new().with_effects(),
            Some(ExecuteTransactionRequestType::WaitForLocalExecution),
        )
        .await?;

    println!("Success. Transaction sent.");
    println!("The Counter::increment function should run on the network and emit an");
    println!("event from package {}", package_id);
    Ok(())
}
