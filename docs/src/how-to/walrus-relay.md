# Walrus Relay

Walrus Relay provides a local HTTP proxy for Walrus upload services, enabling transparent access to Walrus upload endpoints through suibase-daemon.

Unlike the publisher service which stores data directly to Walrus, the relay service acts as a proxy that forwards requests to remote Walrus upload endpoints while providing local request statistics and configuration management.

## Enabling

Enable Walrus Relay for a specific network:

```bash
testnet wal-relay enable
mainnet wal-relay enable
```

This updates the `walrus_relay_enabled: true` setting in your workdir's `suibase.yaml` configuration.

## Starting

The relay starts automatically when you start the workdir services:

```bash
testnet start
mainnet start
```

The relay process runs alongside the suibase-daemon and requires the daemon to be running.

## Disabling

Disable Walrus Relay for a specific network:

```bash
testnet wal-relay disable
mainnet wal-relay disable
```

This updates the `walrus_relay_enabled: false` setting in your workdir's `suibase.yaml` configuration.

## Statistics

View request statistics:

```bash
testnet wal-relay stats
mainnet wal-relay stats
```

Clear accumulated statistics:

```bash
testnet wal-relay clear
mainnet wal-relay clear
```

## How to connect?

Connect your applications to these suibase-daemon proxy ports:

**Testnet**: `http://localhost:45852`
**Mainnet**: `http://localhost:45853`

All HTTP requests are forwarded transparently to the underlying Walrus upload relay service while maintaining full API compatibility.

## Status

Check the current relay status:

```bash
testnet wal-relay status
mainnet wal-relay status
```

Status can be: OK, DOWN, DISABLED, STOPPED, NOT RUNNING, or INITIALIZING.