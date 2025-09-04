# Walrus Relay

Walrus Relay delegates high-bandwidth operations to a backend server, enabling mobile and web apps to store data on Walrus without the burden of uploading to multiple storage nodes directly.

Suibase provides a local relay process for testing your applications before using production services. The binary used are downloaded from Mysten Lab, and are therefore compatible with the official networks.

**Resources:**
- [Walrus SDK](https://sdk.mystenlabs.com/walrus)
- [Upload relay docs](https://docs.wal.app/operator-guide/upload-relay.html)
- [TypeScript SDK upgrade blog](https://www.walrus.xyz/blog/typescript-sdk-upload-relay-upgrade)

**Examples:**
- [Full web app](https://github.com/MystenLabs/walrus-sdk-example-app) ([live demo](https://relay.wal.app/))
- [Single blob upload](https://github.com/MystenLabs/ts-sdks/blob/main/packages/walrus/examples/upload-relay/write-blob.ts)


## Enabling / Starting

Enable Walrus Relay for the network you intend to use. Only testnet and mainnet are supported:

```bash
testnet wal-relay enable
mainnet wal-relay enable
```

The relay starts/stops automatically alongside the workdir services:

```bash
testnet start
testnet stop
```

You can monitor the relay status:

```bash
testnet wal-relay status
mainnet wal-relay status
```

## How to connect?

Connect your applications to these local ports:

**Testnet**: `http://localhost:45852`
**Mainnet**: `http://localhost:45853`

Specify these in the host field of the Mysten SDK:

Example:
```typescript
const client = new SuiClient({
	url: getFullnodeUrl('testnet'),
	network: 'testnet',
}).$extend(
	WalrusClient.experimental_asClientExtension({
		uploadRelay: {
			host: 'http://localhost:45852',
		},
	}),
);
```
## Statistics

View request statistics for a given workdir with:

```bash
testnet wal-relay stats
```

You can clear all stats with:

```bash
testnet wal-relay clear
```

## Disabling

Disable Walrus Relay for a specific network:

```bash
testnet wal-relay disable
mainnet wal-relay disable
```

## Upgrading

It is recommended to regularly upgrade the binaries to match the latest networks:

```bash
testnet update
mainnet update
```

You might want to regularly update suibase itself with ```~/suibase/update```