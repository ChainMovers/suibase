---
title: Sui GraphQL RPC
contributors: true
editLink: true
---

## Facts

::: tip Fact Sheet

- Sui GraphQL RPC is currently in **_beta_**
- Sui GraphQL RPC beta operates on both testnet and mainnet at this time
  - testnet: `https://sui-testnet.mystenlabs.com/graphql`
  - mainnet: `https://sui-mainnet.mystenlabs.com/graphql`
  - devnet not currently supported
  - streaming not currently supported
- Sui GraphQL RPC will eventually _replace_ the JSON RPC
- Sui support and constraints defined [Here](https://docs.sui.io/references/sui-api/beta-graph-ql#using-sui-graphql-rpc)
- PySui support for Sui GraphQL RPC:
    - Release 0.50.0 includes an 'experimental' implementation, subject to change
    - Provides Synchronous and asynchronous GraphQL clients
    - Read queries, DryRun and Execute transactions are supported
    - Introduces `QueryNodes` that are the equivalent to pysui `Builders`
    - Parity of QueryNodes to Builders is complete
    - Exposes ability for developers to write their own GraphQL queries
    - `SuiConfiguration` must point to either Sui's `testnet` or `mainnet` RPC URLs
    - pysui GraphQL documentation is in the [Docs](https://pysui.readthedocs.io/en/latest/graphql.html)
:::

## Generating GraphQL schema

::: code-tabs

@tab sui

```shell
NA
```

@tab pysui

```python
    from pysui import PysuiConfiguration, SyncGqlClient

    def main():
        """Dump Sui GraphQL Schema."""
        # Initialize synchronous client
        cfg = PysuiConfiguration(group_name=PysuiConfiguration.SUI_GQL_RPC_GROUP )
        client_init = SyncGqlClient(pysui_config=cfg,write_schema=True)
        print(f"Schema dumped to: {client_init.base_schema_version}.graqhql`")

    if __name__ == "__main__":
        main()

```
:::

## Query example 1

For pysui there are 3 common ways to create a query. This demonstrates **_using QueryNodes (predefined queries as part of pysui SDK)_**

::: code-tabs

@tab sui

```shell
NA at this time
```

@tab pysui

```python
    """Development example."""

    from pysui import PysuiConfiguration, handle_result, SyncGqlClient
    import pysui.sui.sui_pgql.pgql_query as qn


    def main(client: SyncGqlClient):
        """Fetch 0x2::sui::SUI (default) for owner."""
        # GetCoins defaults to '0x2::sui::SUI' coin type so great for owners gas listing
        qres = client.execute_query_node(
            with_node=qn.GetCoins(
                owner="0x00878369f475a454939af7b84cdd981515b1329f159a1aeb9bf0f8899e00083a"
            )
        )
        # 1. QueryNode results are mapped to dataclasses/dataclasses-json
        print(qres.result_data.to_json(indent=2))

        # 2. Or get the data through handle_result
        # print(handle_result(qres).to_json(indent=2))

    if __name__ == "__main__":
        # Initialize synchronous client
        cfg = PysuiConfiguration(group_name=PysuiConfiguration.SUI_GQL_RPC_GROUP )
        client_init = SyncGqlClient(pysui_config=cfg,write_schema=False)
        main(client_init)
```
:::

## DryRun example 1

This demonstrates performing a DryRun of a transaction block

::: code-tabs

@tab sui

```shell
NA at this time
```

@tab pysui

```python
    #
    """DryRun a TransactionBlock."""

    import base64
    from pysui.sui.sui_pgql.clients import SuiGQLClient
    from pysui.sui.sui_txn import SyncTransaction
    import pysui.sui.sui_pgql.pgql_query as qn
    from pysui import SuiConfig

    def handle_result(result: SuiRpcResult) -> SuiRpcResult:
        """."""
        if result.is_ok():
            if hasattr(result.result_data, "to_json"):
                print(result.result_data.to_json(indent=2))
            else:
                print(result.result_data)
        else:
            print(result.result_string)
            if result.result_data and hasattr(result.result_data, "to_json"):
                print(result.result_data.to_json(indent=2))
            else:
                print(result.result_data)
        return result

    def main(client: SuiGQLClient):
        """Execute a dry run with TransactionData where gas and budget set by txer."""
        txer = SuiTransaction(client=client)
        scres = txer.split_coin(coin=txer.gas, amounts=[1000000000])
        txer.transfer_objects(transfers=scres, recipient=client.config.active_address)

        tx_b64 = base64.b64encode(txer.transaction_data().serialize()).decode()
        print(tx_b64)
        handle_result(
            client.execute_query_node(with_node=qn.DryRunTransaction(tx_bytestr=tx_b64))
        )

    if __name__ == "__main__":
        # Initialize synchronous client (must be mainnet or testnet)
        cfg = PysuiConfiguration(group_name=PysuiConfiguration.SUI_GQL_RPC_GROUP )
        client_init = SyncGqlClient(pysui_config=cfg,write_schema=False)
        main(client_init)
```
:::

## DryRun example 2

This demonstrates performing a DryRun of a transactions kind

::: code-tabs

@tab sui

```shell
NA at this time
```

@tab pysui

```python
    #
    """DryRun a TransactionBlock's TransactionKind."""

    import base64
    from pysui.sui.sui_pgql.clients import SuiGQLClient
    from pysui.sui.sui_txn import SyncTransaction
    import pysui.sui.sui_pgql.pgql_query as qn
    from pysui import SuiConfig

    def handle_result(result: SuiRpcResult) -> SuiRpcResult:
        """."""
        if result.is_ok():
            if hasattr(result.result_data, "to_json"):
                print(result.result_data.to_json(indent=2))
            else:
                print(result.result_data)
        else:
            print(result.result_string)
            if result.result_data and hasattr(result.result_data, "to_json"):
                print(result.result_data.to_json(indent=2))
            else:
                print(result.result_data)
        return result


    def main(client: SuiGQLClient):
        """Execute a dry run with TransactionKind where meta data is set by caller."""
        txer = SuiTransaction(client=client)
        scres = txer.split_coin(coin=txer.gas, amounts=[1000000000])
        txer.transfer_objects(transfers=scres, recipient=client.config.active_address)

        tx_b64 = base64.b64encode(txer.raw_kind().serialize()).decode()
        handle_result(
            client.execute_query_node(with_node=qn.DryRunTransactionKind(tx_bytestr=tx_b64))
        )

    if __name__ == "__main__":
        # Initialize synchronous client (must be mainnet or testnet)
        cfg = PysuiConfiguration(group_name=PysuiConfiguration.SUI_GQL_RPC_GROUP )
        client_init = SyncGqlClient(pysui_config=cfg,write_schema=False)
        main(client_init)
```
:::

## Execute example

This demonstrates performing a transaction execution.

::: code-tabs

@tab sui

```shell
NA at this time
```

@tab pysui

```python
    #
    """Execute a TransactionBlock."""

    import base64
    from pysui.sui.sui_pgql.clients import SuiGQLClient
    from pysui.sui.sui_txn import SyncTransaction
    import pysui.sui.sui_pgql.pgql_query as qn
    from pysui import SuiConfig

    def handle_result(result: SuiRpcResult) -> SuiRpcResult:
        """."""
        if result.is_ok():
            if hasattr(result.result_data, "to_json"):
                print(result.result_data.to_json(indent=2))
            else:
                print(result.result_data)
        else:
            print(result.result_string)
            if result.result_data and hasattr(result.result_data, "to_json"):
                print(result.result_data.to_json(indent=2))
            else:
                print(result.result_data)
        return result


    def main(client: SuiGQLClient):
        """Execute a transaction.

        The result contains the digest of the transaction which can then be queried
        for details
        """
        txer: SuiTransaction = SuiTransaction(client=client)
        scres = txer.split_coin(coin=txer.gas, amounts=[1000000000])
        txer.transfer_objects(transfers=scres, recipient=client.config.active_address)
        txdict = txer.build_and_sign()
        handle_result(client.execute_query_node(with_node=qn.ExecuteTransaction(**txdict)))

    if __name__ == "__main__":
        # Initialize synchronous client (must be mainnet or testnet)
        cfg = PysuiConfiguration(group_name=PysuiConfiguration.SUI_GQL_RPC_GROUP )
        client_init = SyncGqlClient(pysui_config=cfg,write_schema=False)
        main(client_init)
```
:::

## Query example 2

For pysui there are 3 common ways to create a query. This demonstrates **_using a query string_**

::: code-tabs

@tab sui

```shell
# basic query
curl -X POST https://graphql-beta.mainnet.sui.io \
     --header "Content-Type: application/json" \
     --data '{
          "query": "query { epoch { referenceGasPrice } }"
     }'
# query with variables
curl -X POST https://graphql-beta.mainnet.sui.io \
     --header "Content-Type: application/json" \
     --data '{
          "query": "query ($epochID: Int!) { epoch(id: $epochID) { referenceGasPrice } }", "variables": { "epochID": 123 }
     }'
```

@tab pysui

```python
    #
    """Query using a query string."""

    from pysui.sui.sui_pgql.clients import SuiGQLClient
    from pysui import SuiConfig

    def main(client: SuiGQLClient):
        """Execute a static string query."""
        _QUERY = """
            query {
                chainIdentifier
                checkpointConnection (last: 1) {
                    nodes {
                        sequenceNumber
                        timestamp
                    }
                }
                serviceConfig {
                    enabledFeatures
                    maxQueryDepth
                    maxQueryNodes
                    maxDbQueryCost
                    maxPageSize
                    requestTimeoutMs
                    maxQueryPayloadSize
                }
            protocolConfig {
                protocolVersion
                configs {
                    key
                    value
                }
                featureFlags {
                    key
                    value
                }
                }
            }
        """
        qres = client.execute_query(with_string=_QUERY)
        print(qres)

    if __name__ == "__main__":
        cfg = PysuiConfiguration(group_name=PysuiConfiguration.SUI_GQL_RPC_GROUP )
        client_init = SyncGqlClient(pysui_config=cfg,write_schema=False)
        main(client_init)

```
:::

## Query example 3

For pysui there are 3 common ways to create a query. This demonstrates **_using [gql](https://github.com/graphql-python/gql) the underlying GraphQL library_** to generate a DocumentNode

::: code-tabs

@tab sui

```shell
NA
```

@tab pysui

```python
    #
    """Query using gql DocumentNode."""
    from gql import gql
    from pysui.sui.sui_pgql.clients import SuiGQLClient
    from pysui import SuiConfig

    def main(client: SuiGQLClient):
        """Execute a compiled string into DocumentNode."""
        _QUERY = # Same query string as used Query example 2
        qres = client.execute_query(with_document_node=gql(_QUERY))
        print(qres)

    if __name__ == "__main__":
        # Initialize synchronous client (must be mainnet or testnet)
        cfg = PysuiConfiguration(group_name=PysuiConfiguration.SUI_GQL_RPC_GROUP )
        client_init = SyncGqlClient(pysui_config=cfg,write_schema=False)
        main(client_init)

```
:::