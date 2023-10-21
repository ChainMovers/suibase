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

"""Demonstrate coin and balance information."""

import json

from pysui import SuiConfig, SyncClient
from pysui.sui.sui_clients.common import handle_result
from pysui.sui.sui_builders.get_builders import GetAllCoins
from pysui.sui.sui_txresults.single_tx import SuiCoinObjects


def coin(client: SyncClient) -> None:
    """Summarize, by address by coin type, count of coin and balance

    It organizes the information in a dict structure:
    {
        address : {
            coin_type A: [count, total_balance] # Coin type value is a list
            coin_type B: [count, total_balance]
        }
    }

    Args:
        client (SyncClient): The interface to the Sui RPC API.
    """
    summary = {}
    for address in client.config.addresses:
        coin_type_list: SuiCoinObjects = handle_result(
            client.execute(GetAllCoins(owner=address))
        )
        coin_collection = {}
        for coinage in coin_type_list.data:
            if coinage.coin_type in coin_collection:
                inner_list = coin_collection[coinage.coin_type]
                inner_list[0] = inner_list[0] + 1
                inner_list[1] = inner_list[1] + f" {coinage.balance} "
            else:
                coin_collection[coinage.coin_type] = [1, coinage.balance]
        summary[address] = coin_collection
    print(json.dumps(summary, indent=2))


def main(client: SyncClient):
    """Entry point for demo."""
    addy_keypair = client.config.keypair_for_address(client.config.active_address)
    print(
        f"Active address: {client.config.active_address} public-key: {addy_keypair.public_key}"
    )
    coin(client)


if __name__ == "__main__":
    main(SyncClient(SuiConfig.sui_base_config()))
