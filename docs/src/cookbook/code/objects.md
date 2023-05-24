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

```

:::
