---
title: Programmable Transaction
order: 4
contributors: true
editLink: true
---

## Introduction

Prior to Sui 0.28.x, to submit transactions with mutliple commands one was required to invoke the,
recently renamed, `unsafe_batchTransaction`. This is somewhat limited to allowing only calls to
`public entry fun` on contracts (a.k.a. move calls) and transfers.

Starting around Sui version 0.28.x MystenLabs introduced 'programmable transactions' which expanded
the capability of structuring multiple-diverse commands, sharing results between commands, lifting the
limitations of calling only `public entry fun` to now include any `public fun` contract functions and much more.

The current MystenLabs documentation (using Typescript examples) can be found [Here](https://docs.sui.io/devnet/build/prog-trans-ts-sdk)

## This document

The purpose of this guide is to add **general**, language agnostic, information about programmable transactions (herein referred to simply as 'transaction' or 'transactions').

## What is a Transaction?

- Transactions may contain one or more [commands](#what-are-commands)
- Transactions support multiple [signers](#signing-transactions)
- If one command in a transaction fails, the whole transaction fails
- Transactions are inspectable (`sui_devInspectTransactionBlock` ) and can be dry-run (`sui_dryRunTransactionBlock`) as well
- End points (i.e. devnet, testnet, etc.) are configurable

### What are Commands

Commands are a single unit of execution to which you can add many in a single transaction:

- Some of the SDK in the references come with 'out of the box' commands, such as `split`,`merge` and `transfer`
- Commands can contain calls to Sui contracts/packages (i.e. move calls)
- Move calls are not limited to `public entry` functions of the contract, calls to any `public` function are supported
- Commands are run _sequentially_
- Command inputs may be simple (numbers, strings, addresses) or objects (coins, NFTs, shared objects, etc.)
- Typically non-object inputs are often called 'pure' wheras objects are called, well, 'objects'
- Inputs may be collections (i.e. vectors, lists or arrays) of 'pure' types
- Collections of objects are supported through 'making a Move vector', the results of which can be used as input
  to subesquent commands
- Results of commands may be used as inputs to subsequent commands
- Not all commands return a re-usable results. For example: `transfer` does not return a reusable result
- Results may be a single result or an array/list of results

### Known Command Restrictions

- Commands can only operate with mutiple objects for which the primary sender can sign. In other words, if one command is
  operating with address 'A' owned objects, a subequent command can not include address 'B' owned objects as there is no
  way to include another signer. This restriction, and the associated signing liimitation, is reported to be in review to hopefully ease this constraint
- Collections are limited to a depth level of 16 (e.g. `vector<vector<vector<.....>>>`)

### Signing Transactions

At the time of this writing, a maximum of two (2) signers are allowable:

1. The sender, sometimes referred to as the primary signer (can be a MultiSig)
2. A sponsor if the payment for the transaction (gas) is provided by another gas object owner (can be a MultiSig)

## Additional References

Programmable transactions are supported in multiple programming languagges. Known at the time of this writing:

- Typescript: MystenLabs TS SDK
- Rust: MystenLab Rust SDK
- Python: [pysui](https://github.com/FrankC01/pysui)
