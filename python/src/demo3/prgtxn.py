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
"""Demonstrate Programmable Transactions."""

from pysui.abstracts import SignatureScheme
from pysui.sui.sui_types.address import SuiAddress
from pysui.sui.sui_config import SuiConfig
from pysui.sui.sui_clients.sync_client import SuiClient
from pysui.sui.sui_clients.transaction import SuiTransaction
from pysui.sui.sui_txresults.single_tx import SuiCoinObject
from pysui.sui.sui_clients.common import handle_result


from src.common.demo_utils import first_address_for_keytype


def main(client: SuiClient):
    """Entry point for demo.

    This demonstrates using pysui Programmable Transaction (SuiTransaction).
    If finds a address that is ed25519 and one that is secp256k1 and transfers a
    coin from the former to the latter.

    Remember to have `localnet start` running before executing and `localnet set-active`.
    """
    # Get the from (source) and to (recpient) address (ignore the keypair in return tuple)
    from_address, _ = first_address_for_keytype(client, SignatureScheme.ED25519)
    to_address, _ = first_address_for_keytype(client, SignatureScheme.SECP256K1)

    # Setup the Transaction Builder using the client
    # By default, the 'sender' is set to client.config.active-address
    tx_builder = SuiTransaction(client)
    # We reset sender to the 'from_address'
    from_address = SuiAddress(from_address)
    tx_builder.signer_block.sender = from_address

    # Get the first coin that the 'from_address' has
    gasses: list[SuiCoinObject] = handle_result(client.get_gas(from_address)).data
    a_coin: SuiCoinObject = gasses[0]
    # Get it's balance and convert to int
    a_coin_balance = int(a_coin.balance)

    print(f"Transferring 50% of coin: {a_coin.coin_object_id} from address: {from_address} to address: {to_address}")
    # Construct a split coin for 50% of a_coin
    # We want the result as input into the subsequent transfer
    split_coin = tx_builder.split_coin(coin=a_coin.coin_object_id, amounts=int(a_coin_balance / 2))
    # Construct a transfer to send the result of splitting out the coin
    # to the recipient
    tx_builder.transfer_objects(transfers=split_coin, recipient=SuiAddress(to_address))

    # An alternative is to combine:
    # tx_builder.transfer_objects(transfers=tx_builder.split_coin(
    #     coin=a_coin.coin_object_id,
    #     amounts=int(a_coin_balance/2), recipient=SuiAddress(to_address))

    # Lets see the transaction structural representation as JSON
    # UNCOMMENT TO SEE
    # print(tx_builder.raw_kind().to_json(indent=2))

    # Lets run it through inspection and view results
    # print(tx_builder.inspect_all().to_json(indent=2))

    # Lets execute it and check results
    # Signer and gas object will be satisfied by the builder
    # COMMENT THESE LINES IF YOU UNCOMMENT THE OPTIONS ABOVE
    result = tx_builder.execute(gas_budget="2000000")
    if result.is_ok():
        print(result.result_data.to_json(indent=2))


if __name__ == "__main__":
    main(SuiClient(SuiConfig.sui_base_config()))
