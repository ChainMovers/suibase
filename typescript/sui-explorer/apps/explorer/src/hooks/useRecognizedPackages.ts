// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import {
  SUI_FRAMEWORK_ADDRESS,
  SUI_SYSTEM_ADDRESS,
} from "@mysten/sui.js/utils";

import { useNetwork } from "~/context";
import { Network } from "~/utils/api/DefaultRpcClient";

const DEFAULT_RECOGNIZED_PACKAGES = [SUI_FRAMEWORK_ADDRESS, SUI_SYSTEM_ADDRESS];

export function useRecognizedPackages() {
  const [network] = useNetwork();

  const recognizedPackages = DEFAULT_RECOGNIZED_PACKAGES;

  // Our recognized package list is currently only available on mainnet
  return network === Network.MAINNET
    ? recognizedPackages
    : DEFAULT_RECOGNIZED_PACKAGES;
}
