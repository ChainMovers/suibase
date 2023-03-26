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

"""Common low level utilities shared across demo apps."""

import os
from pathlib import Path

_SUI_BASE_ACTIVE: str = "~/sui-base/workdirs/active/config"


def sui_base_config() -> Path:
    """Attempt to load a Sui valid configuration from sui-base.

    Returns:
        Path: Fully qualified path to client.yaml or None if not valid
    """
    # Have the system expand path and resolve symlinks
    client_yaml = Path(os.readlink(
        os.path.expanduser(_SUI_BASE_ACTIVE))) / "client.yaml"
    # If all works out
    if client_yaml.exists():
        return client_yaml
    print(f"{client_yaml} can not resolve sui-base configuration")
    return None


if __name__ == "__main__":
    pass
