---
title: Transactions
contributors: true
editLink: true
---

## Facts

::: tip Fact Sheet
:::

Suggested subjects:

- How to transfer an object
- How to transfer Sui
- How to merge coins
- How to publish a module

## How to transfer an object

::: code-tabs

@tab:active CLI

```shell
# Transfer an object
sui client transfer --to <ADDRESS> --object-id <OBJECT_ID> --gas-budget <GAS_BUDGET>
```

@tab Python

```python
from pysui.sui.sui_clients.sync_client import SuiClient
from pysui.sui.sui_config import SuiConfig
from pysui.sui.sui_clients.transaction import SuiTransaction
from pysui.sui.sui_types.address import SuiAddress

def test_tb_transfer_obj(client: SuiClient = None):
    """Use Transaction Buider to transfer object."""
    # Setup client
    client = client if client else SuiClient(SuiConfig.default_config())
    # Instantiate transaction block builder
    txer = SuiTransaction(client)
    # Identify an object
    primary_coin = "0x9f8150343f6e0357e76ebc4256aa59223a21dc824e63367461df3562081bbb90"
    # Identify a recipient
    recipient = SuiAddress("0xa9fe7b9cab7ce187c768a9b16e95dbc5953a99ec461067a73a6b1c4288873e28")
    print(f"From {client.config.active_address} to {recipient} obj {primary_coin}")
    # Build the transaction
    txer.transfer_objects(
        transfers=[primary_coin],
        recipient=recipient,
    )
    # Execute the transaction
    tx_result = txer.execute(gas_budget="100000")
    if tx_result.is_ok():
        if hasattr(tx_result.result_data, "to_json"):
            print(tx_result.result_data.to_json(indent=2))
        else:
            print(tx_result.result_data)
    else:
        print(tx_result.result_string)
```

@tab TS

```ts
import {
  Ed25519Keypair,
  Connection,
  JsonRpcProvider,
  RawSigner,
  TransactionBlock,
} from "@mysten/sui.js";

// Set a provider
const connection = new Connection({
  fullnode: "http://127.0.0.1:9000",
});

// Generate a new Ed25519 Keypair
const keypair = new Ed25519Keypair();

// Connect to the provider
const provider = new JsonRpcProvider(connection);

// Instantiate RawSigner object
const signer = new RawSigner(keypair, provider);

// Instantiate TransactionBlock object
const tx = new TransactionBlock();

// Build the transfer object
tx.transferObjects(
  [
    tx.object(
      "0xe19739da1a701eadc21683c5b127e62b553e833e8a15a4f292f4f48b4afea3f2"
    ),
  ],
  tx.pure("0x1d20dcdb2bca4f508ea9613994683eb4e76e9c4ed371169677c1be02aaf0b12a")
);

// Perform the object transfer
const result = await signer.signAndExecuteTransactionBlock({
  transactionBlock: tx,
});

// Print the output
console.log({ result });
```

:::

## How to transfer Sui

::: code-tabs

@tab:active CLI

```shell
# Transfer SUI, and pay gas with the same SUI coin object. 
# If amount is specified, only the amount is transferred; 
# otherwise the entire object is transferred

sui client transfer-sui --to <ADDRESS> --sui-coin-object-id <SUI_COIN_OBJECT_ID> --gas-budget <GAS_BUDGET>
```

@tab Python

```python
from pysui.sui.sui_clients.sync_client import SuiClient
from pysui.sui.sui_config import SuiConfig
from pysui.sui.sui_clients.transaction import SuiTransaction
from pysui.sui.sui_types.address import SuiAddress

def test_tb_transfer_sui(client: SuiClient = None):
    """Use Transaction Buider to transfer Sui object."""
    # Setup client
    client = client if client else SuiClient(SuiConfig.default_config())
    # Instantiate transaction block builder
    txer = SuiTransaction(client)
    # Identify a Sui coin object
    primary_coin = "0x9f8150343f6e0357e76ebc4256aa59223a21dc824e63367461df3562081bbb90"
    # Transfer some mists from coin to myself
    txer.transfer_sui(recipient=client.config.active_address, from_coin=primary_coin, amount=100000)
    # Execute
    tx_result = txer.execute(gas_budget="100000")
    if tx_result.is_ok():
        if hasattr(tx_result.result_data, "to_json"):
            print(tx_result.result_data.to_json(indent=2))
        else:
            print(tx_result.result_data)
    else:
        print(tx_result.result_string)

```

@tab TS

```ts
import {
  Ed25519Keypair,
  Connection,
  JsonRpcProvider,
  RawSigner,
  TransactionBlock,
} from "@mysten/sui.js";

// Generate a new Keypair
const keypair = new Ed25519Keypair();

// Set a provider
const connection = new Connection({
  fullnode: "http://127.0.0.1:9000",
});

// Connect to provider
const provider = new JsonRpcProvider(connection);

// Instantiate RawSigner object
const signer = new RawSigner(keypair, provider);

// Instantiate TransactionBlock object
const tx = new TransactionBlock();

// Set 1 Sui to be sent
const [coin] = tx.splitCoins(tx.gas, [tx.pure(1_000_000_000)]);

tx.transferObjects(
  [coin],
  tx.pure("0x8bab471b0b2e69ac5051c58bbbf81159c4c9d42bf7a58d4f795ecfb12c968506")
);

// Perform SUI transfer
const result = await signer.signAndExecuteTransactionBlock({
  transactionBlock: tx,
});

// Print output
console.log({ result });
```

:::

## How to merge coins

::: code-tabs

@tab:active CLI

```shell
# Merge two coin objects into one coin
sui client merge-coin --primary-coin <PRIMARY_COIN> --coin-to-merge <COIN_TO_MERGE> --gas-budget <GAS_BUDGET>
```

@tab Python

```python
from pysui.sui.sui_clients.sync_client import SuiClient
from pysui.sui.sui_config import SuiConfig
from pysui.sui.sui_clients.transaction import SuiTransaction
from pysui.sui.sui_types.address import SuiAddress

def test_tb_merge_to_gas(client: SuiClient = None):
    """Merge all coins but 1 (gas) to gas."""
    # Setup client
    client = client if client else SuiClient(SuiConfig.default_config())
    # Ensure Sender independent of active-address
    sender = SuiAddress("0xb0d73b5bcb842853c3e5367325ccfd15a81a141842f9d798c793f2d597cc65c5")
    # Instantiate transaction block builder
    txer = SuiTransaction(client, initial_sender=sender)
    # Get senders coin inventory and ensure there is at least 2
    e_coins: SuiCoinObjects = handle_result(client.get_gas(sender))
    assert len(e_coins.data) > 1. "Nothing to merge"
    # Merge all other coins but 1st (gas) to  gas
    txer.merge_coins(merge_to=txer.gas, merge_from=e_coins.data[1:])
    # Execute
    tx_result = txer.execute(gas_budget="100000")
    if tx_result.is_ok():
        if hasattr(tx_result.result_data, "to_json"):
            print(tx_result.result_data.to_json(indent=2))
        else:
            print(tx_result.result_data)
    else:
        print(tx_result.result_string)


```

@tab TS

```ts
import {
  Ed25519Keypair,
  Connection,
  JsonRpcProvider,
  RawSigner,
  TransactionBlock,
} from "@mysten/sui.js";

// Generate a new Keypair
const keypair = new Ed25519Keypair();

// Set a provider
const connection = new Connection({
  fullnode: "http://127.0.0.1:9000",
});

// Connect to provider
const provider = new JsonRpcProvider(connection);

// Instantiate RawSigner object
const signer = new RawSigner(keypair, provider);

// Instantiate TransactionBlock object
const tx = new TransactionBlock();

// Build merge transaction
tx.mergeCoins(
  tx.object(
    "0x5406c80f34fb770d9cd4226ddc6208164d3c52dbccdf84a6805aa66c0ef8f01f"
  ),
  [
    tx.object(
      "0x918af8a3580b1b9562c0fddaf102b482d51a5806f4485b831aca6feec04f7c3f"
    ),
  ]
);

// Perform the merge
const result = await signer.signAndExecuteTransactionBlock({
  transactionBlock: tx,
});

// Print the output
console.log({ result });
```

:::
