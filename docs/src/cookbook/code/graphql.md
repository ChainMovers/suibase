---
title: Sui GraphQL RPC
contributors: true
editLink: true
---

## Facts

::: tip Fact Sheet

- Sui GraphQL RPC is currently in **_beta_**
- Sui GraphQL RPC beta operates on a snapshot of data with timestamps:
  - testnet: "2023-12-16T19:07:30.993Z"
  - mainnet: "2023-11-21T22:03:27.667Z"
  - devnet not currently supported
- Sui GraphQL RPC will eventually _replace_ the JSON RPC
- Sui support and constraints defined [Here](https://docs.sui.io/references/sui-api/beta-graph-ql#using-sui-graphql-rpc)
- PySui support for Sui GraphQL RPC:
    - Release 0.50.0 includes an 'experimental' implementation, subject to change
    - Provides Synchronous and asynchronous GraphQL clients
    - Only 'read' queries are supported at the time of this writing
    - Introduces `QueryNodes` that are the equivalent to pysui `Builders`
    - Parity of QueryNodes to Builders is ongoing
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

        print("Schema dumped to: `latest_schemaVERSION.graqhql`")

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