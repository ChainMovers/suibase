---
title: Network Environments
contributors: true
editLink: true
---

## Connecting to specific network environment

When you are working on Sui development, you will need to connect to a specific Sui Full node on a Sui network.

- mainnet https://fullnode.mainnet.sui.io:443/
- devnet https://fullnode.devnet.sui.io:443/
- testnet https://fullnode.testnet.sui.io:443/

::: code-tabs

@tab CLI

```shell
# Display network environments 
sui client envs

# Switch to a network environment
sui client switch --env <ENVIRONMENT>

# Add a network environment
sui client new-env --alias <ENVIRONMENT> --rpc <RPC-URL>
```

@tab Python

```python
# pysui uses the environment alias settings in client.yaml
# Use CLI command sui client switch --env <ALIAS_NAME> to change network
```

@tab:active TypeScript

```ts
import { SuiClient, getFullnodeUrl } from '@mysten/sui.js/client';

// create a client object connected to localnet
const client = new SuiClient({ url: getFullnodeUrl('localnet') });

```

:::

## Getting Test Sui

If you want to test transactions on Sui network you need first to get Sui coins in your wallet. To receive test Sui in your wallet you have to make a request to the faucet server.

::: code-tabs

@tab CLI

```shell
To be done. Add your contribution here.
```

@tab Python

```python
To be done. Add your contribution here.
```

@tab:active TypeScript

```ts
import { JsonRpcProvider, devnetConnection } from "@mysten/sui.js";

// connect to Devnet
const provider = new JsonRpcProvider(devnetConnection);

// get test Sui from the DevNet faucet server
await provider.requestSuiFromFaucet(
  "0x8bab471b0b2e69ac5051c58bbbf81159c4c9d42bf7a58d4f795ecfb12c968506"
);
```

:::
