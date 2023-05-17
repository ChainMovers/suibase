---
title: MultiSig
contributors: true
editLink: true
---

## Facts

::: tip Fact Sheet

- MultiSigs provide another level of security governance by requiring more than one key to sign transactions
- MultiSigs are constructed with 2 or more keypairs, each with their own weight
- Construction also requires a threshold. The threshold is checked at signature creation time and if the
  associated sum of 'weights' of the keys provided are less than the threshold, signing will fail
- MultiSigs have a unique address
- You can send objects to the MultiSig address like any other Sui address
- To manipulate objects owned by the MultiSig address, you must sign with 1 or more of the keys of the constructed MultiSig
- Creation and signing for MultiSig from CLI can be found on [Sui Docs](https://docs.sui.io/testnet/learn/cryptography/sui-multisig)
- Unlike other keys that are stored in `sui.keystore`, MultiSigs are not persisted.

:::

### Create a MultiSig

::: code-tabs

@tab:active Rust

```Rust
To be done. Add your contribution here.
```

@tab Python

```python
from pysui.sui.sui_crypto import MultiSig, SuiKeyPair

# Create a list of keypairs.
sui_keypairs:list [SuiKeyPair] = [keypair1, keypair2, keypair3]
# Create equal number of weights
keypair_weights:list [int] = [1,3,4]
# Threshold to meet when signing
threshold:int = 4

# Create the multisig that consists of 3 keys with
# associated weights and a signing threshold

msig = MultiSig(sui_keypairs,keypair_weights, threshold)

print(f"MultiSig unique address: {msig.address}")

```

@tab TS

```ts
To be done. Add your contribution here.
```

:::

### Sign with a MultiSig

::: code-tabs

@tab:active Rust

```Rust
To be done. Add your contribution here.
```

@tab Python

```python
from pysui.sui.sui_crypto import MultiSig
from pysui.sui.sui_clients.sync_client import SuiClient
from pysui.sui.sui_builders.exec_builders import Pay

def sui_pay_ms(
    client: SuiClient,
    to_addy: SuiAddress,
    msig: MultiSig, amount: int
    ) -> SuiRpcResult:
    """Pay some balance from MultiSig address to some other address."""
    pay_builder = Pay(
        signer=msig.address,
        input_coins=[ObjectID("0x1ad571e2cbdb813045f88022b02f52d55c26b3118088a293ac3063241b9f6470")],
        recipients=[to_addy],
        amounts=[SuiString(amount)],
        gas=ObjectID("0x9de1dade0323c16c2b20f7fd2fce362041467f3d42c90f3f8445f8a8b435558d"),
        gas_budget="2000000",
    )

    # Sign with a subject of keys. In this example, we just borrow two of the 3
    # keys we used to construct the multisig. Their combinded weights = 4 which
    # meets the threshold specified at MultiSig creation

    result = client.execute_with_multisig(pay_builder, msig, msig.public_keys[0:2])
    if result.is_ok():
        print(result.result_data.to_json(indent=2))
    else:
        print(result.result_string)
    return result
```

@tab TS

```ts
To be done. Add your contribution here.
```

:::
