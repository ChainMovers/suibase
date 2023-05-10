---
title: Fast Transactions
order: 6
contributors: true
editLink: true
---

A notable Sui feature is its capability to handle fast "Simple transaction" at scale. These are for single-owner objects that do not require relatively more costly/slower consensus.


::: danger Danger

Fast transaction have to be done with care to avoid equivocations. This can result in the dreaded "quorum failure" that locks your owned object until the end of an epoch. This guide should help you design your app to benefit from fast transactions AND remain reliable.
:::

## Don't do this

May be, the most important to understand is what not to do:
- Do not initiate multiple transaction with the same owned object at the same time.
- Do not use the same coin with multiple simple transaction at the same time.


## From Slow To Fast
TODO Refer to example transforming a slow design into fast ones (think I saw one in the Sui repo?)

## Distinct Coins
TODO Explain how distinct coin management is crucial to parallel processing.

## Faucet
TODO Explain how the Sui faucet work as a design example.
