// Subscribe and display all events from a Sui localnet
//
// type 'cargo run events' to see this code in action.
//
// type 'cargo run count' in another terminal to trigger a "counter increment" event.
//
use colored::Colorize;
use futures::StreamExt;
use sui_base_helper::SuiBaseHelper;
use sui_json_rpc_types::EventFilter;
use sui_sdk::SuiClientBuilder;

// Subscribe for all events with the Sui network.
//
// This function loop until Ctrl-C or error.
//
pub async fn display_events_loop() -> Result<(), anyhow::Error> {
    let suibase = SuiBaseHelper::new();
    suibase.select_workdir("active")?;

    let rpc_url = suibase.rpc_url()?;
    let ws_url = suibase.ws_url()?;

    println!("Using sui-base workdir [{}]", suibase.workdir()?);
    println!("Connecting to Sui network at [{}]", ws_url);

    let sui = SuiClientBuilder::default()
        .ws_url(ws_url)
        .build(rpc_url)
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
        }
    }
}
