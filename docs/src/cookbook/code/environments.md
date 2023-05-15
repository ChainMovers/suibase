---
title: Network Environments
contributors: true
editLink: true
---

## Connecting to specific network environment

When you are working on Sui development, you will need to connect to a specific Sui Full node on a Sui network. 

- mainnet https://fullnode.mainnet.sui.io/
- devnet https://fullnode.devnet.sui.io
- testnet https://fullnode.testnet.sui.io/

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
import { JsonRpcProvider, devnetConnection } from '@mysten/sui.js';
// connect to Devnet

const provider = new JsonRpcProvider(devnetConnection);
```

:::
