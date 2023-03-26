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
from src.common.demo_utils import handle_result
from src.common.low_level_utils import sui_base_config
from pysui.sui.sui_config import SuiConfig
from pysui.sui.sui_clients.common import SuiRpcResult
from pysui.sui.sui_clients.sync_client import SuiClient
from pysui.sui.sui_builders.get_builders import GetSuiSystemState, GetValidators
from pysui.sui.sui_txresults.single_tx import SuiSystemState


def _stats_0271(client: SuiClient):
    """Show system info for 0.27.1

    Args:
        client (SuiClient): The interface to the Sui RPC API.
    """
    # Get system information
    sysinfo: SuiSystemState = handle_result(
        client.execute(GetSuiSystemState()))
    dtime = datetime.utcfromtimestamp(
        sysinfo.epoch_start_timestamp_ms/1000)
    print(
        f"Current Epoch: {sysinfo.epoch}, running since UTC: {dtime.strftime('%Y-%m-%d %H:%M:%S')}")
    print(f'Reference gas price: {sysinfo.reference_gas_price} mist')
    # Validator information
    print(
        f"Validators stake {sysinfo.validators.validator_stake} mist"
    )
    validators = handle_result(client.execute(
        GetValidators())).validator_metadata
    print(
        f"Active Validators: {len(validators)}")
    for vmd in validators:
        print(f"[{vmd.name}] address:  {vmd.sui_address}")


def _stats_0281(client: SuiClient):
    """Show system info for 0.28.1

    Args:
        client (SuiClient): The interface to the Sui RPC API
    """
    print(f"{client.rpc_version} not handled yet")


def main(client: SuiClient):
    """Entry point for demo."""
    print(f"\nSui client RPC version{client.rpc_version}")
    # Information not related to some version
    addy_keypair = client.config.keypair_for_address(
        client.config.active_address)
    print(
        f"Active address: {client.config.active_address} public-key: {addy_keypair.public_key}")
    match client.rpc_version:
        case "0.27.1":
            _stats_0271(client)
        case "0.28.1":
            _stats_0281(client)
        case _:
            print(f"{client.rpc_version} not handled")


if __name__ == "__main__":
    base_config = sui_base_config()
    if base_config:
        main(SuiClient(SuiConfig.from_config_file(base_config)))
