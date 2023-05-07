---
title: Fullnode
order: 5
contributors: true
editLink: true
---

First thing first, ask yourself which network (localnet/testnet/devnet) should be use and if you really need a fullnode.

#### Is localnet enough?

In most case, localnet is a great way to develop and test dApps. It is fast, reliable, deterministic with unlimited funds.

Consider using devnet/testnet only when you have reach the point of depending on other dApps, you are established and need a public presence or as the last steps before going on mainnet.

Keep in mind that your development may come to a halt when devnet/testnet is being wiped out, and you have to wait on someone else module publication. Consider this versus getting in the habit of publishing all your dependencies on your own localnet...

If your goal is to learn Move, then definitely start with localnet. For suibase users, it is as simple as doing `$ localnet start`. Even for advanced user, localnet provides the most efficient edit/publish/test development cycle.


#### Why a fullnode?

Expect public/free RPC fullnodes to be normally throttled. You will experience "timeouts" and slow transaction time.

One solution is to pay for a fullnode (a service provided by many validators). The alternative is to run your own.

You may also need to run your own testnet/devnet/mainnet "indexer" if critical to your dApps. This is useful for maintaining a database of events and shared object creation.

#### Setup

Make sure to check the latest on the sui discord "node-operator" channel (including pin messages). The moderators are very helpful and responsive.

Installation procedure : https://docs.sui.io/build/fullnode

#### Monitoring

SuiMon is recommended for simple, reliable monitoring over CLI. You will get as a bonus great support from "K | BartestneT" on the sui discord: https://github.com/bartosian/suimon

Node monitoring (Web Browser):
 * https://www.scale3labs.com/check/sui
 * https://node.sui.zvalid.com/
 * https://sui.explorers.guru/node

Telegram Notification in case your node is not catching up:
 https://t.me/sui_checker_bot

#### More?

You found something not right on this guide? Want to add something about "Prometheus+Grafana" monitoring? Please consider becoming a contributor. It is easy and will help the Sui community.