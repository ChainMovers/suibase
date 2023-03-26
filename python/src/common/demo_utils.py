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
