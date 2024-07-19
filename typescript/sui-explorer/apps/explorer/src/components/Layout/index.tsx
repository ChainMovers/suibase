// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { SuiClientProvider, WalletProvider } from "@mysten/dapp-kit";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { Fragment } from "react";
import { Toaster, resolveValue, type ToastType } from "react-hot-toast";
import { Outlet, ScrollRestoration } from 'react-router-dom';

import { KioskClientProvider } from "@mysten/core/src/components/KioskClientProvider";
import { NetworkContext, useNetwork } from "~/context";
import { Banner, type BannerProps } from "~/ui/Banner";
import {
  NetworkConfigs,
  createSuiClient,
  type Network,
} from "~/utils/api/DefaultRpcClient";

const toastVariants: Partial<Record<ToastType, BannerProps["variant"]>> = {
  success: "positive",
  error: "error",
};

export function Layout() {
  const [network, setNetwork] = useNetwork();

  return (
    // NOTE: We set a top-level key here to force the entire react tree to be re-created when the network changes:
    <Fragment key={network}>
      <ScrollRestoration />
      <SuiClientProvider
        networks={NetworkConfigs}
        createClient={createSuiClient}
        network={network as Network}
        onNetworkChange={setNetwork}
      >
        <WalletProvider autoConnect enableUnsafeBurner={import.meta.env.DEV}>
          <KioskClientProvider>
            <NetworkContext.Provider value={[network, setNetwork]}>
              <Outlet />
              <Toaster
                position="bottom-center"
                gutter={8}
                containerStyle={{
                  top: 40,
                  left: 40,
                  bottom: 40,
                  right: 40,
                }}
                toastOptions={{
                  duration: 4000,
                }}
              >
                {(toast) => (
                  <Banner shadow border variant={toastVariants[toast.type]}>
                    {resolveValue(toast.message, toast)}
                  </Banner>
                )}
              </Toaster>
              <ReactQueryDevtools />
            </NetworkContext.Provider>
          </KioskClientProvider>
        </WalletProvider>
      </SuiClientProvider>
    </Fragment>
  );
}
