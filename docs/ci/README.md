DO NOT MODIFY the files in this directory

**What are they for?**

They are used by GitHub Action for CI deployment.

Only the site maintainers should modify these.

This is done such that minor difference in
installation in someone's setup does not clash
with the intent from the site maintainers.

**How to update?**
Site maintainer execute the following at ~/suibase/docs:
```bash
 cp package.json ci/package.json.ci
 cp pnpm-lock.yaml ci/pnpm-lock.yaml.ci
```
