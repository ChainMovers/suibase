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

```CLI
To be done. Add your contribution here.
```

@tab Python

```python
To be done. Add your contribution here.
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
        '0xe19739da1a701eadc21683c5b127e62b553e833e8a15a4f292f4f48b4afea3f2',
    ),
],
    tx.pure('0x1d20dcdb2bca4f508ea9613994683eb4e76e9c4ed371169677c1be02aaf0b12a'),
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

```CLI
To be done. Add your contribution here.
```

@tab Python

```python
To be done. Add your contribution here.
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

tx.transferObjects([coin], tx.pure("0x8bab471b0b2e69ac5051c58bbbf81159c4c9d42bf7a58d4f795ecfb12c968506"));

// Perform SUI transfer
const result = await signer.signAndExecuteTransactionBlock({
    transactionBlock: tx,
});

// Print output
console.log({ result });

```

:::