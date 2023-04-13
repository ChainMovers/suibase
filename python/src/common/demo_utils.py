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

"""Common demo utilities shared across demo apps."""
import sys
from typing import Any
from pysui.sui.sui_clients.common import SuiRpcResult
from pysui.abstracts import SignatureScheme, KeyPair
from pysui.sui.sui_clients.sync_client import SuiClient
from pysui.sui.sui_types.address import SuiAddress


def default_handler(result: SuiRpcResult) -> Any:
    """Default result handler used if not specified in handle_result

    Args:
        result (SuiRpcResult): The result from a pysui call to client

    Returns:
        Any: The data returned with the result
    """
    if result.is_ok():
        return result.result_data
    print(f"Error in result: {result.result_string}")
    sys.exit(-1)


def handle_result(from_cmd: SuiRpcResult, handler=default_handler) -> Any:
    """Takes a SuiRpcResult and invoked handler.

    Args:
        from_cmd (SuiRpcResult): The result of some SuiClient call
        handler (fn(SuiRpcResult), optional): A callable handler function that takes a SuiRpcResult. Defaults to default_handler.
    """
    assert callable(handler), "Invalid 'handler' argument"
    assert isinstance(from_cmd, SuiRpcResult), "Invalid 'from_command' return"
    return handler(from_cmd)


def first_address_for_keytype(client: SuiClient, keytype: SignatureScheme) -> tuple[SuiAddress, KeyPair]:
    """Get a SuiAddress and KeyPair tuple for specific keytype.

    Args:
        client (SuiClient): Use the configuration from a specific SuiClient provider
        keytype (SignatureScheme): Indicate the key type to filter on

    Raises:
        ValueError: No match found

    Returns:
        tuple[SuiAddress, KeyPair]: A matching address and keypair of first found tuple
    """
    filtered: tuple[SuiAddress, KeyPair] = [(k, v) for (k, v) in client.config.addresses_and_keys.items()
                                            if v.scheme == keytype]
    if filtered:
        return filtered[0]
    raise ValueError(f"No keypair type of {keytype.as_str()}")
