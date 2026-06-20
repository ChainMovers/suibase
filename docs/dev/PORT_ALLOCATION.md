# Suibase default port allocation

Reference for the TCP ports Suibase assigns by default in
`scripts/defaults/<workdir>/suibase.yaml`. All ports are **configurable** per workdir;
this documents the *defaults* and the convention to follow when adding a new service so
ports stay collision-free and predictable.

## The two conventions

1. **Range = service family.** Each kind of service lives in a numeric range:

   | Range | Family | Examples |
   |---|---|---|
   | `9xxx` | Sui-native (Mysten defaults) | Sui fullnode RPC `9000`, sui-faucet `9123` |
   | `443xx` | Suibase core infrastructure | multi-link proxy `4434x`, explorer `44380`, DTP `44397/44398`, suibase-daemon API `44399` |
   | `458xx` | Walrus HTTP services | walrus-relay local `4580x` / metrics `4581x` / proxy `4585x`; **sb-local (Walrus API) `4584x`** |

2. **Rightmost digit = workdir index**, for a service that exists per workdir:

   | Workdir | Index | Example: proxy `4434x` | Example: walrus-relay proxy `4585x` |
   |---|---|---|---|
   | localnet | **0** | `44340` | `45850` (reserved) |
   | devnet | **1** | `44341` | — |
   | testnet | **2** | `44342` | `45852` |
   | mainnet | **3** | `44343` | `45853` |

   A service gets its own *band* (the first three or four digits), then the per-workdir
   instances differ only in the last digit. Localnet-only services just use the `…0` slot.

## Current assignments

| Service | localnet | devnet | testnet | mainnet | Notes |
|---|---|---|---|---|---|
| Sui fullnode RPC | `9000` | — | — | — | Mysten default |
| sui-faucet | `9123` | `9124` | `9125` | `9125` | Mysten-derived; only localnet runs a local faucet process |
| Multi-link proxy | `44340` | `44341` | `44342` | `44343` | suibase-daemon |
| Explorer | `44380` | — | — | — | localnet only |
| DTP web / api | `44397` / `44398` | — | — | — | localnet only |
| suibase-daemon API | `44399` | (shared) | (shared) | (shared) | one daemon, all workdirs |
| walrus-relay local / metrics / proxy | — | — | `45802`/`45812`/`45852` | `45803`/`45813`/`45853` | testnet/mainnet only |
| **sb-local (Walrus API)** | **`45840`** | (`45841`) | (`45842`) | (`45843`) | localnet only today; `4584x` band reserved |

## Why sb-local is `45840`

`sb-local` is the localnet **Walrus aggregator/publisher HTTP server** — the localnet
counterpart of the walrus-relay, which already owns the `458xx` "Walrus services" range.
So sb-local belongs in `458xx`, not in `443xx` (suibase core infra) and not in `9xxx`
(where `9124` would have duplicated the devnet/cargobin faucet default). It takes a fresh
band `4584x` (clear of the relay's `4580x`/`4581x`/`4585x`) and the localnet slot `…0`,
i.e. **`45840`** — leaving `45841/45842/45843` reserved should it ever expand per-workdir.

## Adding a new service

1. Pick the family range (`443xx` core infra, `458xx` Walrus, …).
2. Take a free *band* within it (don't reuse another service's band).
3. Assign per-workdir with the rightmost-digit-as-workdir-index rule (localnet `…0`).
4. Add it to every relevant `scripts/defaults/<workdir>/suibase.yaml` and update the table
   above.
