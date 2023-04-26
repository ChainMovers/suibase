---
title: Build Faster
order: 3
contributors: true
editLink: true
---

## Use a local Sui repo

If you build often, then repeating local file access is obviously faster than remote (and more reliable).

If you use the Rust SDK, replace your "git" dependencies with "path".

For Move dependencies replace "git" dependencies with "local".

For suibase users, see [here](./../../how-to/scripts.md#faster-rust-and-move-build) to re-use its local repo already downloaded.

## Build only what you need

If you care only for the client, then do not build the whole thing.
Do `cargo build -p sui` instead of `cargo build`

## Parallel Linker

Some build steps are not optimized for parallelism. Notably, you can see this with `top` on Linux (by pressing <kbd>1</kbd>) and you will see only one core busy while the linker is running.

One trick that _may_ help is the parallel linker [Mold](https://github.com/rui314/mold).

After installation, you can enable for Rust by creating a `config.toml`. The following was verified to work for Sui built on Ubuntu:

```
$ cat ~/.cargo/config.toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

The performance gain varies widely, you have to try for yourself. Do not expect 10x faster... it accelerates only the link phase. Furthermore, the performance gap versus more recent GNU/LLVM linker release is closing.

## How does my build time compare?

See some profiling below.

Measurements are for clean build of sui and sui-faucet only.

::: details Steps for measuring

With suibase, do the following to get one measurement:

```shell
$ localnet delete
$ localnet update
```

If you do not have suibase, then do the following for the first measurements:

```shell
$ git clone -b devnet https://github.com/MystenLabs/sui.git
$ cd sui
$ cargo build -p sui -p sui-faucet
```

... and get additional measurements with:

```shell
$ cargo clean
$ cargo build -p sui -p sui-faucet
```

:::

**Modern Linux**<br>

```text
Low : Finished dev [unoptimized + debuginfo] target(s) in 2m 55s
High: Finished dev [unoptimized + debuginfo] target(s) in 2m 55s

Intel i7-13700K (16 Cores), 64 GB, NVMe PCIe 4
Ubuntu 22.10, Sui 0.31.2
Suibase 0.1.2
GCC 12.2 / Mold 1.11
```

**M1 MAX macosx**<br>

```text
Low : Finished dev [unoptimized + debuginfo] target(s) in 4m 23
High: Finished dev [unoptimized + debuginfo] target(s) in 4m 24s

Apple M1 Max
macOS Ventura 13.3.1 (22E261), Sui 0.31.2
Suibase 0.1.2
Apple clang version 14.0.3
rustc 1.68.2
```

**Old Server (~2013) Windows 10 WSL2**<br>

```text
Low : Finished dev [unoptimized + debuginfo] target(s) in 8m 06s
High: Finished dev [unoptimized + debuginfo] target(s) in 8m 20s

2xIntel Xeon E5-2697v2@2.7GHz(24 Cores), 64 GB, NVMe PCIe 3
WSL2 config: 32 VCore, 48 GB
Ubuntu 22.04, Sui 0.31.2
Suibase 0.1.2
GCC 11.3 / Mold 1.11
```
