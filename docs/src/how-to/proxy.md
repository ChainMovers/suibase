---
title: "Proxy Server ( Multi-Link RPC )"
order: 2
---

A local gRPC server that makes your queries to Sui more reliable with:
  - Selection of fastest RPC server currently available.
  - Automatic retry on failure (when safe against transaction execution repetition).
  - Load balancing across multiple RPC servers (to minimize rate-limiting).

## How to use it?
Have your applications query toward the addresses of the local proxy server:
|  Network    |      Proxy Link           |
| :---------: | :-----------------------: |
| localnet    | ```http://localhost:44340```    |
| devnet      | ```http://localhost:44341```    |
| testnet     | ```http://localhost:44342```    |
| mainnet     | ```http://localhost:44343```    |

The proxy forwards/retries/distributes your queries among all the healthy RPC nodes configured.

All suibase scripts (and the corresponding client.yaml) are already configured to use the proxy server by default.

Useful related workdir commands are start/stop/status and links (e.g. ```devnet start```, ```testnet links``` etc...).

## Monitoring RPC Links
The proxy runs in background when you start one of localnet, devnet, testnet or mainnet (e.g. ```devnet start```).

The workdir ```links``` command (e.g. ```testnet links```) shows status for all configured RPC servers.

<img :src="$withBase('/assets/testnet-links.png')" alt="testnet links"><br>

::: details Details on cumulative request stats
Each **user query** is counted only once in one of the following metrics depending upon its outcome.<br>
**Success first attempt**: One healthy RPC node was selected and the request succeeded immediately.<br>
**Success after retry**: One healthy RPC node was selected but failed, the request eventually succeeded after retries with one or more other servers (when safe to retry). From a user perspective, the request succeeded as if a single RPC node was used.<br>
**Failure bad request**: It was determined that the request was malformed and would not succeed with any RPC nodes. Therefore the request is not retried and the failure is reported immediately. Check the response for a hint on how to fix the request.<br>
**Failure others**: All other scenarios when the response was not a gRPC success. The proxy response will have more information.<br>
:::

::: details Details on individual server stats
**Status**: Either OK or DOWN. Goes DOWN on any failure related to the server not responding. Every 15 seconds the proxy does a health check toward every node to verify availability and measure response time.<br>
**Health%**: Varies from -100% (most unhealthy) to 100% (healthiest). This is intended for  relative comparison between all servers. The score factors duration of healthy state or failures, frequency of retries and rate limitation etc... any positive value is considered healthy (Status OK).<br>
**Load%**: Distribution of the **user** request among all nodes. The sum of the load% is 100%. Health check queries are not included in this stat.<br>
**RespT**: The time between the health check query and the response time. Average of the last 20 queries (the result from the most recent query has more weight).<br>
**Success%**: Ratio of **user** requests that were successful after the server was selected and considered healthy. Will typically be 100%, a value below 100% signifies the server did unexpectedly fail on a user query (Note: the query might have been retried on another server and succeeded from a user perspective).<br>
:::


## RPC links configuration
Out of the box, suibase comes with a set of public RPC links known to be operating. This is maintained by the community and updated whenever you do ```~/suibase/update```.

You can add your own RPC links by editing your workdir's suibase.yaml file.
Example with two RPC nodes:

``` yaml
links:
  - alias: "sui.io"
    rpc: "https://fullnode.mainnet.sui.io:443"
    priority: 10
  - alias: "suiscan.xyz"
    rpc: "https://rpc-mainnet.suiscan.xyz:443"
    metrics: "https://rpc-mainnet.suiscan.xyz/metrics"
    priority: 20
```
- The indentation is important (two spaces before the '-').
- 'alias' and 'rpc' are mandatory. All others are optional.

::: details All Links Parameters
**alias**
Mandatory unique name for the link. Recommended less than 20 characters.

**rpc**
Mandatory RPC node address. Typically ````https://<node name>:443````

**metric**
The metric address. Not commonly provided by public nodes. For future use. You can specify it, but currently not used. [ Default = None ]

**priority**
A preference order when selecting between multiple servers. It is used, as an example, when the proxy server is initializing and the health of the remote RPC nodes is not yet all known. A node with a smaller priority number might be selected first. All default links provided by suibase are in the 10 to 20 range [ Default = 20 ]
:::

## Upgrade
The proxy server updates and restarts as needed when you do ```~/suibase/update```.

## Stopping and Disabling
Use the workdir ```stop``` command (e.g. ```devnet stop```) to temporarily disable the proxy services for a specific workdir.<br>
Disabling is also configurable by adding ```proxy_enabled: false``` to a suibase.yaml for a specific workdir.<br>
You can disable for all workdirs at once by adding ```proxy_enabled: false``` to:<br>```~/suibase/workdirs/common/suibase.yaml```

## Adding RPC Services with API Key
This is an example of how to add services from https://www.shinami.com/

Add to ```~/suibase/workdirs/testnet/suibase.yaml```:
``` yaml
links:
  - alias: "shinami.com"
    rpc: "https://api.shinami.com:443/node/v1/sui_testnet_xxxxxxxxx"
    priority: 10
```

Replace "sui_testnet_xxxxxxxxx" with the API key specific to your shinami account (info is in their web app).

Do the same for mainnet in ```~/suibase/workdirs/mainnet/suibase.yaml```.

The proxy server automatically detects and applies the changes after you save the .yaml file.

Many other commercial RPC services can be added in the same way.