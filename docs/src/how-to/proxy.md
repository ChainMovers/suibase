---
title: "Multi-Link RPC ( Proxy Server )"
order: 2
---

Makes your JSON-RPC queries more reliable with:
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

The proxy forward/retry/distribute automatically your queries among all the healthy RPC nodes configured.

All suibase scripts (and the corresponding client.yaml) are already configured to use the proxy server by default.

Useful related workdir commands are start/stop/status and links (e.g. "devnet start", "testnet links" etc...).

## Monitoring RPC Links
The proxy server runs in background automatically when you start one of localnet, devnet, testnet or mainnet (e.g. 'devnet start').

The workdir ```links``` command (e.g. ```testnet links```) shows the health of all its configured RPC nodes.

<img :src="$withBase('/assets/testnet-links.png')" alt="testnet links"><br>

::: details Details on perfomance monitoring
The success/failure of every query affects a health score for the RPC node. Every 15 seconds the suibase daemon will do a "health check" test query toward every node to keep a fresh view of their availability.
**RespT** measure the time between the health check query and the response time. The average is an exponential moving average of the last 20 queries (the result from the most recent query has more weight).
**Load** is the percent of the user queries that were handled by this node. The health check query are not included in this stat.
:::

## RPC links configuration
You can customize your own RPC links by editing your workdir's suibase.yaml file.

By default, suibase come with a set of links. You can add more links by adding a 'links' section. Example with two RPC nodes:
``` yaml
links:
  - alias: "sui.io"
    rpc: "https://fullnode.mainnet.sui.io:443"
    ws: "wss://fullnode.mainnet.sui.io:443"
    priority: 10  
  - alias: "suiscan.xyz"
    rpc: "https://rpc-mainnet.suiscan.xyz:443"
    metrics: "https://rpc-mainnet.suiscan.xyz/metrics"
    ws: "wss://rpc-mainnet.suiscan.xyz/websocket"
    priority: 20
```
- The indentation is important (two spaces before the '-').
- 'alias' and 'rpc' are mandatory. All others are optional. 

::: details All Links Parameters
**alias**
Mandatory unique name for the link. Recommended less than 20 characters.

**rpc**
Mandatory RPC node address. Typically ````https://<node name>:443````

**ws**
Websocket address. For future use. You can specify it, but currently not used. [ Default = None ]

**metric**
The metric address. Not commonly provided by public nodes. For future use. You can specify it, but currently not used. [ Default = None ]

**priority**
A preference order when selecting between multiple servers. It is used, as an example, when the proxy server is initializing and the health of the remote RPC nodes are not yet all known. A node with a smaller priority number might be selected first. All default links provided by suibase are in 10 to 20 range [ Default = 20 ]
:::

## Upgrade
The proxy server update and restart as needed when you do '~/suibase/update'.

## Stopping and Disabling
Use the ```workdir stop``` command (e.g. ```devnet stop```) to stop the proxy services (also the daemon will no longuer monitor the health of the RPC nodes).
Disabling is also configureable by adding 'proxy_enabled: false' to a suibase.yaml in a specific workdir.
You can disable for all workdirs at once by adding it to ```~/suibase/workdirs/common/suibase.yaml```

