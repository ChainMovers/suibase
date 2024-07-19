// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { useSuiClientQuery } from '@mysten/dapp-kit';
import { useContext } from 'react';

import { NetworkSelect, type NetworkOption } from "~/ui/header/NetworkSelect";
import { NetworkContext } from "../../context";
import { Network } from "../../utils/api/DefaultRpcClient";

export default function WrappedNetworkSelect() {
	const [network, setNetwork] = useContext(NetworkContext);
	const { data } = useSuiClientQuery('getLatestSuiSystemState');
	const { data: binaryVersion } = useSuiClientQuery('getRpcApiVersion');

	const networks = [{ id: Network.LOCAL, label: "Local" }].filter(
    Boolean
  ) as NetworkOption[];

	return (
    <NetworkSelect
      value={network}
      onChange={(networkId) => {
        setNetwork(networkId);
      }}
      networks={networks}
      version={data?.protocolVersion}
      binaryVersion={binaryVersion}
    />
  );
}
