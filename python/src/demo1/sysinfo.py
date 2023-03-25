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
from src.common.low_level_utils import sui_base_config
from pysui.sui.sui_config import SuiConfig
from pysui.sui.sui_clients.common import SuiRpcResult
from pysui.sui.sui_clients.sync_client import SuiClient
from pysui.sui.sui_builders.get_builders import GetSuiSystemState
from pysui.sui.sui_txresults.single_tx import SuiSystemState


def _stats_027(client: SuiClient):
    """Show system info for 0.27.1

    Args:
        client (SuiClient): _description_
    """
    # Get system information
    result: SuiRpcResult = client.execute(GetSuiSystemState())
    if result.is_ok():
        sysinfo: SuiSystemState = result.result_data
        # Epoch information, epoch time and current gas price
        dtime = datetime.utcfromtimestamp(
            sysinfo.epoch_start_timestamp_ms/1000)
        print(
            f"Current Epoch: {sysinfo.epoch}, running since UTC: {dtime.strftime('%Y-%m-%d %H:%M:%S')}")
        print(f'Reference gas price: {sysinfo.reference_gas_price} mist')
        # Validator information
        print(
            f"Active Validators: {len(sysinfo.validators.active_validators)}")
        print(
            f"Validators stake {sysinfo.validators.validator_stake} mist"
        )


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
            _stats_027(client)
        case _:
            print(f"{client.rpc_version} not handled yet")


if __name__ == "__main__":
    base_config = sui_base_config()
    # tcfg = json.loads(base_config.read_bytes())
    # print(tcfg)
    if base_config:
        main(SuiClient(SuiConfig.from_config_file(base_config)))
