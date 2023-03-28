// Subscribe and display all events from a Sui localnet
//
// type 'cargo run events' to see this code in action.
//
// type 'cargo run count' in another terminal to trigger a "counter increment" event.
//
use colored::Colorize;
use futures::StreamExt;
use sui_json_rpc_types::EventFilter;
use sui_sdk::SuiClientBuilder;

// Subscribe for all events with the Sui network.
//
// This function loop until Ctrl-C or error.
//
pub async fn display_events_loop() -> Result<(), anyhow::Error> {
    // TODO Get URLs from sui-base ( https://github.com/sui-base/sui-base/issues/6 )

    let sui = SuiClientBuilder::default()
        .ws_url("ws://0.0.0.0:9000")
        .build("http://0.0.0.0:9000")
        .await?;

    let mut subscribe_all = sui
        .event_api()
        .subscribe_event(EventFilter::All(vec![]))
        .await?;

    let ready_message = "subscribe_event() success. Listening for all events...";
    println!("{}", ready_message.green());

    loop {
        let nxt = subscribe_all.next().await;

        if nxt.is_none() {
            continue;
        }

        // TODO Make this a bit more "nice, colorful, entertaining"
        if let Ok(env) = nxt.unwrap() {
            println!("Event {:#?}", env);
            /*
            match env.event {
                SuiEvent::Publish { package_id, .. } => {
                    println!("Event new package 0x{} published", package_id.to_hex())
                }

                SuiEvent::MoveEvent { package_id, .. } => {
                    println!("Event emited from package 0x{}", package_id.to_hex())
                }

                SuiEvent::NewObject {
                    package_id,
                    object_id,
                    ..
                } => {
                    println!(
                        "Event new object 0x{} created for package 0x{}",
                        object_id.to_hex(),
                        package_id.to_hex()
                    )
                }

                _ => {}
            }*/
        }
    }
}
