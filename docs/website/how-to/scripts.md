---
hide:
  - toc
---
# Complete list of scripts

This is just a brief intro.

Best way to learn about these scripts is probably just... try them... and "--help".


| Script Name                       | What are they for?                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| lsui<br>dsui<br>tsui<br>          | These scripts are front-ends to Mysten Lab "sui" binaries.<br> They target directly a network, no need to "switch" env.<br><br>  (lsui->localnet, dsui->devnet, tsui->testnet). <br><br>Each script always uses the proper Sui binary+keystore+client.yaml set for the intended network.<br> The scripts are mostly transparent; all their arguments are pass unchanged to a Mysten sui binary.<br><br>Example: '$ lsui client gas'   <-- same as 'sui client gas' when active-env is 'localnet' |
| localnet<br>devnet<br>testnet<br> | The lsui/dsui/tsui scripts are intended to remain as close as possible to Mysten lab sui binary.<br> Consequently, additional services are provided through a different set of scripts.<br><br>Example: "$ localnet faucet all"  <-- This will send Sui coins to every address on your localnet<br>                                                                                                                                                                                              |
| asui                              | You can designate one workdir as "active".<br> This script will call its  corresponding sui binary. This allows multiple independent tool and/or other scripts to coordinate at targeting the same active workdir/network.                                                                                                                                                                                                                                                                       |


