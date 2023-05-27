---
title: Objects
contributors: true
editLink: true
---

## Facts

::: tip Fact Sheet

- Addressses are not objects
- Objects are the instantiation (creation) of a move module's **struct**
- Coins (including gas), other NFT and/or arbitrary structures are objects
- Not all objects are addressable on the chain. To be addressable, **structs** must have the `has key` ability
- Addressable objects are identified by a 32 byte array usually represented as a hex string. For example: **0x002bd2d4aac5da6af372a842baf98590213a6bf4160eb0b46ec0cc3d626b42d3**
- Addressable objects can be queried from client SDK or CLI by it's hex string identifier
- In addition to field values, queried objects also reveal other attributes such as **owner**
- Objects ownership may be one of: _AddressOwner_, _ObjectOwner_, _Shared_ or _Immutable_
- With the exception of objects with _Immutable_ ownership, objects can be modified by the module that created them
- Manipulating the data returned from querying an object has no effect on the chain

:::

## Fetch and Inspect an Object

::: code-tabs

@tab CLI

```shell

sui client object 0x002bd2d4aac5da6af372a842baf98590213a6bf4160eb0b46ec0cc3d626b42d3

```

@tab Python

```python
from pysui.sui.sui_clients.sync_client import SuiClient
from pysui.sui.sui_config import SuiConfig
from pysui.sui.sui_builders.get_builders import GetObject

cfg = SuiConfig.default()
client = SuiClient(cfg)

# The client has a convenience method for fetching objects by ID
result = client.get_object("0x002bd2d4aac5da6af372a842baf98590213a6bf4160eb0b46ec0cc3d626b42d3")
if result.is_ok():
  print(result.result_data.to_json(indent=2))
else:
  print(result.result_string)

# Alternatley you can use a pysui Builder
result = client.execute(GetObject("0x002bd2d4aac5da6af372a842baf98590213a6bf4160eb0b46ec0cc3d626b42d3"))
if result.is_ok():
  print(result.result_data.to_json(indent=2))
else:
  print(result.result_string)


```

@tab TypeScript

```ts
import { JsonRpcProvider, Connection } from "@mysten/sui.js";


// Set a provider
const connection = new Connection({
    fullnode: "http://127.0.0.1:9000",
});

// Connect to provider
const provider = new JsonRpcProvider();

// Fetch object details
const txn = await provider.getObject({
  id: '0xcc2bd176a478baea9a0de7a24cd927661cc6e860d5bacecb9a138ef20dbab231',
  // fetch the object content field
  options: { showContent: true },
});

```

:::

## Fetch Active Address Owner Objects

::: code-tabs

@tab CLI

```shell

sui client objects

```

@tab Python

```python
from pysui.sui.sui_clients.sync_client import SuiClient
from pysui.sui.sui_config import SuiConfig
from pysui.sui.sui_builders.get_builders import GetObjectsOwnedByAddress

cfg = SuiConfig.default()
client = SuiClient(cfg)

# The client has a convenience method for fetching all objects owned by active address
result = client.get_objects()
if result.is_ok():
  for owned_object in result.result_data.data:
    print(owned_object.to_json(indent=2))
else:
  print(result.result_string)

# Alternatley you can use a pysui Builder
result = client.execute(GetObjectsOwnedByAddress("0xa9e2db385f055cc0215a3cde268b76270535b9443807514f183be86926c219f4"))
if result.is_ok():
  for owned_object in result.result_data.data:
    print(owned_object.to_json(indent=2))
else:
  print(result.result_string)


```

@tab TypeScript

```ts
import { JsonRpcProvider, Connection } from "@mysten/sui.js";

// Set a provider
const connection = new Connection({
    fullnode: "http://127.0.0.1:9000",
});

// Connect to provider
const provider = new JsonRpcProvider(connection);

// Get owned objects by an address
const ownedObjects = await provider.getOwnedObjects({
  owner: '0xcc2bd176a478baea9a0de7a24cd927661cc6e860d5bacecb9a138ef20dbab231',
});

// Print owned objects
console.log(ownedObjects.data);
```

:::

## Fetch Multiple Objects

::: code-tabs

@tab CLI

```shell

Not supported

```

@tab Python

```python
from pysui.sui.sui_clients.sync_client import SuiClient
from pysui.sui.sui_config import SuiConfig
from pysui.sui.sui_builders.get_builders import GetMultipleObjects

cfg = SuiConfig.default()
client = SuiClient(cfg)

# The client has a convenience method for fetching multible arbitrary objects
result = client.get_objects_for(
  [
    ObjectID("0x002bd2d4aac5da6af372a842baf98590213a6bf4160eb0b46ec0cc3d626b42d3"),
    ObjectID("0x04fa0b57591b49aa031f18e6a66e98c95e8db31f37e09436eabbd739df59f1bb"),
    #etc
  ])
if result.is_ok():
  for one_of_object in result.result_data:
    print(one_of_object.to_json(indent=2))
else:
  print(result.result_string)

# Alternatley you can use a pysui Builder
result = client.execute(GetMultipleObjects(
  [
    ObjectID("0x002bd2d4aac5da6af372a842baf98590213a6bf4160eb0b46ec0cc3d626b42d3"),
    ObjectID("0x04fa0b57591b49aa031f18e6a66e98c95e8db31f37e09436eabbd739df59f1bb"),
    #etc
  ]))
if result.is_ok():
  for one_of_object in result.result_data:
    print(one_of_object.to_json(indent=2))
else:
  print(result.result_string)


```

@tab TypeScript

```ts
import { JsonRpcProvider, Connection } from "@mysten/sui.js";


// Set a provider
const connection = new Connection({
    fullnode: "http://127.0.0.1:9000",
});

// Connect to provider
const provider = new JsonRpcProvider();

// Fetch multiple object details in one request
const txns = await provider.multiGetObjects({
  ids: [
    '0xcc2bd176a478baea9a0de7a24cd927661cc6e860d5bacecb9a138ef20dbab231',
    '0x9ad3de788483877fe348aef7f6ba3e52b9cfee5f52de0694d36b16a6b50c1429',
  ],
  // only fetch the object type
  options: { showType: true },
});

```

:::

## Object Fetch Options

Sui RPC API allows you to specify options when fetching objects to determine what
kind of information you want returned. The following key values are the _available_ options:

```json
{
  "showType": true,
  "showOwner": true,
  "showPreviousTransaction": true,
  "showDisplay": true,
  "showContent": true,
  "showBcs": true,
  "showStorageRebate": true
}
```

Using the above in your object fetch calls would return _all_ information about the object.
However; you can choose individual flags individually or in combination.

::: code-tabs

@tab CLI

```shell

Not supported

```

@tab Python

```python
from pysui.sui.sui_clients.sync_client import SuiClient
from pysui.sui.sui_config import SuiConfig
from pysui.sui.sui_builders.get_builders import GetObject

def get_object_with_options(client: SuiClient = None):
    """Iterate through various options to display associated results."""
    client = client if client else SuiClient(SuiConfig.default_config())
    target = "0x002bd2d4aac5da6af372a842baf98590213a6bf4160eb0b46ec0cc3d626b42d3"
    # Full list of options. Note that if no options are provided to GetObject
    # or GetMultipleObjects, they default to all options being True
    options: dict = {
        "showType": True,
        "showOwner": True,
        "showPreviousTransaction": True,
        "showDisplay": True,
        "showContent": True,
        "showBcs": True,
        "showStorageRebate": True,
    }

    # Create a list of unique options
    entries = [dict([x]) for x in options.items()]
    # For each options, demonstrate the return content
    for entry in entries:
        print(f"Getting object with option {entry}")
        print(handle_result(
          client.execute(GetObject(object_id=target, options=entry))).to_json(indent=2))
```

@tab TypeScript

```ts

```

:::
