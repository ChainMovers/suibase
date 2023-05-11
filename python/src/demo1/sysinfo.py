# Copyright Frank Castellucci

# Licensed under the Apache License, Version 2.0 (the "License")
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#   http: // www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# -*- coding: utf-8 -*-

"""Demonstrate fetching general system information."""

from datetime import datetime
from pysui.sui.sui_clients.common import handle_result
from pysui.sui.sui_config import SuiConfig
from pysui.sui.sui_clients.sync_client import SuiClient


def _stats_0291(client: SuiClient):
    """Show system info for local node.

    Args:
        client (SuiClient): The interface to the Sui RPC API

    """
    from pysui.sui.sui_builders.get_builders import GetLatestSuiSystemState
    from pysui.sui.sui_txresults.single_tx import SuiLatestSystemState

    sysinfo: SuiLatestSystemState = handle_result(client.execute(GetLatestSuiSystemState()))
    dtime = datetime.utcfromtimestamp(int(sysinfo.epoch_start_timestamp_ms) / 1000)
    print(f"Current Epoch: {sysinfo.epoch}, running since UTC: {dtime.strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"Reference gas price: {sysinfo.reference_gas_price} mist")
    print(f"Active Validators: {len(sysinfo.active_validators)}")
    for vmd in sysinfo.active_validators:
        print(f"[{vmd.name}] address:  {vmd.sui_address} staking balance: {vmd.staking_pool_sui_balance}")


def main(client: SuiClient):
    """Entry point for demo."""
    print(f"\nSui client RPC version {client.rpc_version}")
    # Information not related to some version
    addy_keypair = client.config.keypair_for_address(client.config.active_address)
    print(f"Active address: {client.config.active_address} public-key: {addy_keypair.public_key}")
    _stats_0291(client)


if __name__ == "__main__":
    main(SuiClient(SuiConfig.sui_base_config()))
