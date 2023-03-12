---
hide:
  - toc
---
# References

Sui-Base define a few conventions to coordinate among SDKs, apps and user.

## Filesystem Path Convention

There are 6 <WORKDIR\>:<br> active, localnet, devnet, testnet, mainnet and cargobin

Each <WORKDIR\> has the following components:

| Component      | Purpose                                                                                                                                                         |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| sui-exec       | A script allowing any app to safely call the right sui client+config combination. Use it like you would use the "sui" client from Mysten Lab.                   |
| config         | Directory with Mysten Lab files needed to run the sui client (client.yaml and sui.keystore).                                                                    |
| sui-repo       | A local repo of the Mysten lab sui code for building the client binary, but also for any apps to use the Rust SDK crates for compatibility.                     |
| published-data | Information about last package published from this <WORKDIR\> using sui-base scripts. This can be retrieved through JSON files or through sui-base SDK helpers. |

Applications can expect the components to be always at these **fix** locations:
```
 ~/
 └─ sui-base/
      └─ workdirs/
           └─ <WORKDIR>/
                 ├── sui-exec
                 │
                 ├── config
                 │      ├── client.yaml
                 │      └── sui.keystore
                 │
                 ├── sui-repo
                 │      ├── crates/
                 │      ├── target/
                 │      └── ... complete sui repo (debug built) ...
                 │
                 └── published-data
                        └─ <package name>
                                └─ publish-output.json
```
??? abstract "Official and Complete Path List"
    ~/sui-base/workdirs/<WORKDIR\>/sui-exec<br>
    ~/sui-base/workdirs/<WORKDIR\>/config/client.yaml<br>
    ~/sui-base/workdirs/<WORKDIR\>/config/sui.keystore<br>
    ~/sui-base/workdirs/<WORKDIR\>/sui-repo/<br>
    ~/sui-base/workdirs/<WORKDIR\>/published-data/<PACKAGE_NAME\>/publish-output.json<br>


TODO next:

- What is the "active" workdir?
- What is the "cargobin" workdir?
- How to use the sui-exec script?
- How to use the publish-output.json?

## Sui Client Concurrency Limitation
Explain architecture limitation related to active-address, active-env, switch and such...
