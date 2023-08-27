---
title: Transactions
contributors: true
editLink: true
---

## Facts

::: tip Fact Sheet
:::

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
from pysui.sui.sui_txn.sync_transaction import SuiTransaction
from pysui.sui.sui_types.address import SuiAddress

def test_tb_transfer_obj(client: SuiClient = None):
    """Use Synchronous Transaction Builder to transfer object."""
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
import { SuiClient, getFullnodeUrl } from '@mysten/sui.js/client';
import { Ed25519Keypair } from '@mysten/sui.js/keypairs/ed25519';
import { TransactionBlock } from '@mysten/sui.js/transactions';

// create a client object connected to testnet
const client = new SuiClient({ url: getFullnodeUrl('testnet') });

// Generate a new Ed25519 Keypair
const keypair = new Ed25519Keypair();

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
const result = await client.signAndExecuteTransactionBlock({
  signer: keypair,
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
from pysui.sui.sui_txn.sync_transaction import SuiTransaction
from pysui.sui.sui_types.address import SuiAddress

def test_tb_transfer_sui(client: SuiClient = None):
    """Use Synchronous Transaction Builder to transfer Sui object."""
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
import { SuiClient, getFullnodeUrl } from '@mysten/sui.js/client';
import { Ed25519Keypair } from '@mysten/sui.js/keypairs/ed25519';
import { TransactionBlock } from '@mysten/sui.js/transactions';

// create a client object connected to testnet
const client = new SuiClient({ url: getFullnodeUrl('testnet') });

// Generate a new Keypair
const keypair = new Ed25519Keypair();

// Instantiate TransactionBlock object
const tx = new TransactionBlock();

// Set 1 Sui to be sent
const [coin] = tx.splitCoins(tx.gas, [tx.pure(1_000_000_000)]);

tx.transferObjects(
  [coin],
  tx.pure("0x8bab471b0b2e69ac5051c58bbbf81159c4c9d42bf7a58d4f795ecfb12c968506")
);

// Perform SUI transfer
const result = await client.signAndExecuteTransactionBlock({
  signer: keypair,
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
from pysui.sui.sui_txn.sync_transaction import SuiTransaction
from pysui.sui.sui_types.address import SuiAddress
from pysui.sui.sui_txresults.single_tx import SuiCoinObjects

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
    assert len(e_coins.data) > 1, "Nothing to merge"
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
import { SuiClient, getFullnodeUrl } from '@mysten/sui.js/client';
import { Ed25519Keypair } from '@mysten/sui.js/keypairs/ed25519';
import { TransactionBlock } from '@mysten/sui.js/transactions';

// create a client object connected to testnet
const client = new SuiClient({ url: getFullnodeUrl('testnet') });

// Generate a new Keypair
const keypair = new Ed25519Keypair();

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
const result = await client.signAndExecuteTransactionBlock({
  signer: keypair,
  transactionBlock: tx,
});

// Print the output
console.log({ result });
```

:::

## How to split coins

Split a coin object into multiple coins.

::: code-tabs

@tab:active CLI

```shell
sui client split-coin --coin-id <COIN_ID> --gas-budget <GAS_BUDGET> --amounts <AMOUNTS>
```

@tab Python

```python
from pysui.sui.sui_clients.sync_client import SuiClient
from pysui.sui.sui_config import SuiConfig
from pysui.sui.sui_txn.sync_transaction import SuiTransaction
from pysui.sui.sui_types.address import SuiAddress

def test_tb_split_from_gas(client: SuiClient = None):
    """Split out multiple coins and transfer."""
    client = client if client else SuiClient(SuiConfig.default_config())
    txer = SuiTransaction(client)
    # Split two new coins from gas
    splits:list = txer.split_coin(coin=txer.gas, amounts=[100000000, 100000000])
    # Transfer both the coins to receipient
    txer.transfer_objects(transfers=splits, recipient=SuiAddress("<SOME_SUI_ADDRESS>"))
    # You can also send coins to different address
    # txer.transfer_objects([transfers=splits[0]], recipient=SuiAddress("<SOME_SUI_ADDRESS_0>"))
    # txer.transfer_objects([transfers=splits[1]], recipient=SuiAddress("<SOME_SUI_ADDRESS_1>"))
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
import { SuiClient, getFullnodeUrl } from '@mysten/sui.js/client';
import { Ed25519Keypair } from '@mysten/sui.js/keypairs/ed25519';
import { TransactionBlock } from '@mysten/sui.js/transactions';

// create a client object connected to testnet
const client = new SuiClient({ url: getFullnodeUrl('testnet') });

// Set a MMENONIC
const MNEMONIC = "";

// Get keypair from deriving mnemonic phassphrase
const keypair = Ed25519Keypair.deriveKeypair(MNEMONIC, "m/44'/784'/0'/0'/0'");

// Instantiate TransactionBlock() object
const tx = new TransactionBlock();

// Build splitCoins tx
const [coin] = tx.splitCoins(tx.gas, [tx.pure(1_000_000)]);

// Transfer the objects to a specified address
tx.transferObjects([coin], tx.pure(keypair.getPublicKey().toSuiAddress()));

// Perform the split
const result = await client.signAndExecuteTransactionBlock({
  signer: keypair,
  transactionBlock: tx,
});

// Print the output
console.log({ result });
```

:::

## How to publish a package

::: code-tabs

@tab:active CLI

```shell
sui client publish --gas-budget <GAS_BUDGET> --path <PACKAGE_PATH>
```

@tab Python

```python
from pysui.sui.sui_clients.sync_client import SuiClient
from pysui.sui.sui_config import SuiConfig
from pysui.sui.sui_txn.sync_transaction import SuiTransaction
from pysui.sui.sui_types.address import SuiAddress

def test_tb_publish(client: SuiClient = None):
    """Publish a sui move package."""
    client = client if client else SuiClient(SuiConfig.default_config())
    txer = SuiTransaction(client)
    # Prove a path to the project folder (where the Move.toml lives)
    pcap = txer.publish(project_path="PATH_TO_CONTRACT_PROJECT")
    # Transfer the UpgradeCap to the current active address
    txer.transfer_objects(transfers=[pcap], recipient=client.config.active_address)
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
import { SuiClient, getFullnodeUrl } from '@mysten/sui.js/client';
import { Ed25519Keypair } from '@mysten/sui.js/keypairs/ed25519';
import { TransactionBlock } from '@mysten/sui.js/transactions';

import { execSync } from "child_process";

// create a client object connected to testnet
const client = new SuiClient({ url: getFullnodeUrl('testnet') });

// Set a mnemonic passphrase
const MNEMONIC = "";

// Generate Ed25519 keypair from MNEMONIC
const keypair = Ed25519Keypair.deriveKeypair(MNEMONIC, "m/44'/784'/0'/0'/0'");

// Set the package path
const packagePath = "";

// Set Sui CLI path
const cliPath = "";

// Get modules and dependencies output in JSON format
const { modules, dependencies } = JSON.parse(
  execSync(
    `${cliPath} move build --dump-bytecode-as-base64 --path ${packagePath}`,
    { encoding: "utf-8" }
  )
);

// Instantiate TransactionBlock object
const tx = new TransactionBlock();

// Publish modules and dependencies
const [upgradeCap] = tx.publish({
  modules,
  dependencies,
});

// Transfer object to signer
tx.transferObjects([upgradeCap], tx.pure(await signer.getAddress()));

// Perform the package deployment
const result = await client.signAndExecuteTransactionBlock({
  signer: keypair,
  transactionBlock: tx,
});

// Print the output
console.log({ result });
```

:::

## How to make a Move call

You can make a move call only if you provide a package, module and a function.
The standardized rule of a move call is : `package::module::function`.

::: code-tabs

@tab:active CLI

```shell
sui client call --function function --module module --package <PACKAGE_ID> --gas-budget <GAS_BUDGET>
```

@tab Python

```python
from pysui.sui.sui_types.scalars import ObjectID
from pysui.sui.sui_clients.sync_client import SuiClient
from pysui.sui.sui_config import SuiConfig
from pysui.sui.sui_txn.sync_transaction import SuiTransaction

def test_tb_move_call(client: SuiClient = None):
    """Call a sui contract function."""
    client = client if client else SuiClient(SuiConfig.default_config())
    txer = SuiTransaction(client)
    # The target is a triplet of 'sui_contract_address::contract_module_name::module_function_name'
    # Good to know the signature and whether there is a returned value from the function. For example:
    # entry public set_dynamic_field(Arg0: &mut ServiceTracker, Arg1: &mut TxContext)
    # Note: Function does not have to be of 'entry' type, as long as it is public
    # Note: No need to provide the TxContext object as it is done by the chain before entering contract
    txer.move_call(
        target="0xcc9f55f5403e5df0ec802b3bf6f9849e0fe85ae3fa29c166d066425aa96b6ea9::base::set_dynamic_field",
        arguments=[ObjectID("0xe4b6e6c24adac8f389f4fd15cfd02a7362af3026757eabd46402098b59a2629e")],
    )
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
import { SuiClient, getFullnodeUrl } from '@mysten/sui.js/client';
import { Ed25519Keypair } from '@mysten/sui.js/keypairs/ed25519';
import { TransactionBlock } from '@mysten/sui.js/transactions';

// create a client object connected to testnet
const client = new SuiClient({ url: getFullnodeUrl('testnet') });

// Set a mnemonic passphrase
const MNEMONIC = "";

// Generate Ed25519 keypair from MNEMONIC
const keypair = Ed25519Keypair.deriveKeypair(MNEMONIC, "m/44'/784'/0'/0'/0'");

// Set a package object ID
const packageObjectId = "";

// Instantiate TransactionBlock() object
const tx = new TransactionBlock();

// Move Call without any arguments.
tx.moveCall({
  target: `${packageObjectId}::module::function`,
});

// Move Call with a single argument
tx.moveCall({
  target: `${packageObjectId}::module::function`,
  arguments: [
    tx.pure(
      "0x83e059bce01752a768004cdcc86cf50acf0b47d28802e18226de63fda0023603"
    ),
  ],
});

// Move call with more than 1 argument
tx.moveCall({
  target: `${packageObjectId}::module::function`,
  arguments: [tx.pure("an_argument"), tx.pure("another_argument")],
});

// Perform the move call
const result = await client.signAndExecuteTransactionBlock({
  signer: keypair,
  transactionBlock: tx,
});

// Print the output
console.log({ result });
```

:::

## Transaction Options

Sui RPC API allows you to specify options to control the results returned when
executing a transaction. The following key values are the _available_ options:

```json
{
  "showBalanceChanges": true,
  "showEffects": true,
  "showEvents": true,
  "showInput": true,
  "showObjectChanges": true,
  "showRawInput": true
}
```

Using the above in your transaction execution calls would return _all_ information about the transaction.
However; you can choose individual flags individually or in combination.

::: code-tabs

@tab Python

```python
from pysui.sui.sui_clients.sync_client import SuiClient
from pysui.sui.sui_config import SuiConfig
from pysui.sui.sui_txn.sync_transaction import SuiTransaction
from pysui.sui.sui_clients.common import handle_result
from pysui.sui.sui_builders.get_builders import GetTx

def get_txn_with_options(client: SuiClient, target_digest: str):
    """Iterate through various options to display associated results."""
    options = {
        "showEffects": True,
        "showEvents": True,
        "showBalanceChanges": True,
        "showObjectChanges": True,
        "showRawInput": True,
        "showInput": True,
    }
    # For each options
    entries = [dict([x]) for x in options.items()]
    for entry in entries:
        print(handle_result(client.execute(GetTx(digest=target_digest, options=entry))).to_json(indent=2))

    # Uncomment for full options (defaults to ALL the above)
    # print(handle_result(client.execute(GetTx(digest=target_digest))).to_json(indent=2))

def execution_options(client: SuiClient = None):
    """Setup transaction and inspect options."""
    client = client if client else SuiClient(SuiConfig.default_config())
    # Create simple transaction
    txer = SuiTransaction(client)
    scres = txer.split_coin(coin=txer.gas, amounts=[100000000])
    txer.transfer_objects(transfers=[scres], recipient=client.config.active_address)
    # Execute
    tx_result = txer.execute(gas_budget="100000")
    # Review result
    if tx_result.is_ok():
        get_txn_with_options(client, tx_result.result_data.digest)
    else:
        print(tx_result.result_string)

```

@tab TypeScript

```ts

```

:::
