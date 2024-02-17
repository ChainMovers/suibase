---
title: Sui GraphQL RPC
contributors: true
editLink: true
---

## Facts

::: tip Fact Sheet

- Sui GraphQL RPC is currently in **_beta_**
- Sui GraphQL RPC beta operates on both testnet and mainnet at this time
  - testnet: "https://sui-testnet.mystenlabs.com/graphql"
  - mainnet: "https://sui-mainnet.mystenlabs.com/graphql"
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
    - SuiConfiguration must point to either Sui's `testnet` or `mainnet` RPC URLs
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
    from pysui.sui.sui_pgql.clients import SuiGQLClient
    from pysui import SuiConfig

    def main():
        """Dump Sui GraphQL Schema."""
        # Initialize synchronous client (must be mainnet or testnet)
        client_init = SuiGQLClient(config=SuiConfig.default_config(),write_schema=True)

        print("Schema dumped to: `<NETWORK>_schema-<VERSION>.graqhql`")

    if __name__ == "__main__":
        main()

```
:::

## Query example 1

For pysui there are 3 comon ways to create a query. This demonstrates **_using QueryNodes (predefined queries as part of pysui SDK)_**

::: code-tabs

@tab sui

```shell
NA at this time
```

@tab pysui

```python
    #
    """Query using predefined pysui QueryNode."""

    from pysui.sui.sui_pgql.clients import SuiGQLClient
    import pysui.sui.sui_pgql.pgql_query as qn
    from pysui import SuiConfig

    def main(client: SuiGQLClient):
        """Fetch 0x2::sui::SUI (default) for owner."""
        # GetCoins defaults to '0x2::sui::SUI' coin type so great for owners gas listing
        # Replace <ADDRESS_STRING> with a valid testnet or mainnet address
        qres = client.execute_query(
            with_query_node=qn.GetCoins(
                owner="<ADDRESS_STRING>"
            )
        )
        # Results are mapped to dataclasses/dataclasses-json
        print(qres.to_json(indent=2))

    if __name__ == "__main__":
        # Initialize synchronous client (must be mainnet or testnet)
        client_init = SuiGQLClient(config=SuiConfig.default_config(),write_schema=False)
        main(client_init)
```
:::

## Dryrun example 1

This demonstrates performing a dryRun of a transaction block

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
        if client.chain_environment == "testnet":
            txer = SyncTransaction(client=SyncClient(client.config))
            scres = txer.split_coin(coin=txer.gas, amounts=[1000000000])
            txer.transfer_objects(transfers=scres, recipient=client.config.active_address)

            tx_b64 = base64.b64encode(txer.get_transaction_data().serialize()).decode()
            handle_result(
                client.execute_query(
                    with_query_node=qn.DryRunTransaction(tx_bytestr=tx_b64)
                )
            )

    if __name__ == "__main__":
        # Initialize synchronous client (must be mainnet or testnet)
        client_init = SuiGQLClient(config=SuiConfig.default_config(),write_schema=False)
        main(client_init)
```
:::

## Dryrun example 2

This demonstrates performing a dryRun of a transactions kind

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
        if client.chain_environment == "testnet":
            txer = SyncTransaction(client=SyncClient(client.config))
            scres = txer.split_coin(coin=txer.gas, amounts=[1000000000])
            txer.transfer_objects(transfers=scres, recipient=client.config.active_address)
            # Serialize the TransactionKind which performs faster
            tx_b64 = base64.b64encode(txer.raw_kind().serialize()).decode()
            handle_result(
                client.execute_query(
                    with_query_node=qn.DryRunTransactionKind(tx_bytestr=tx_b64)
                )
            )

    if __name__ == "__main__":
        # Initialize synchronous client (must be mainnet or testnet)
        client_init = SuiGQLClient(config=SuiConfig.default_config(),write_schema=False)
        main(client_init)
```
:::

## Execute example

This demonstrates performing a execution of a transactions

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
        if client.chain_environment == "testnet":
            rpc_client = SyncClient(client.config)
            txer = SyncTransaction(client=rpc_client)
            scres = txer.split_coin(coin=txer.gas, amounts=[1000000000])
            txer.transfer_objects(transfers=scres, recipient=client.config.active_address)
            tx_b64 = txer.deferred_execution(run_verification=True)
            sig_array = txer.signer_block.get_signatures(client=rpc_client, tx_bytes=tx_b64)
            # sig_array is a SuiArray which wraps a list, we want the list only
            rsig_array = [x.value for x in sig_array.array]
            handle_result(
                client.execute_query(
                    with_query_node=qn.ExecuteTransaction(
                        tx_bytestr=tx_b64, sig_array=rsig_array
                    )
                )
            )

    if __name__ == "__main__":
        # Initialize synchronous client (must be mainnet or testnet)
        client_init = SuiGQLClient(config=SuiConfig.default_config(),write_schema=False)
        main(client_init)
```
:::

## Query example 2

For pysui there are 3 comon ways to create a query. This demonstrates **_using a query string_**

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
        client_init = SuiGQLClient(config=SuiConfig.default_config(),write_schema=False)
        main(client_init)

```
:::

## Query example 3

For pysui there are 3 comon ways to create a query. This demonstrates **_using [gql](https://github.com/graphql-python/gql) the underlying GraphQL library_** to generate a DocumentNode

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
        client_init = SuiGQLClient(config=SuiConfig.default_config(),write_schema=False)
        main(client_init)

```
:::