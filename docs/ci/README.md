DO NOT MODIFY the files in this directory
Do not update these files.

**What are they for?**

They are used by github action for CI deployment.

Only the maintainer of the website update these.

This is done such that minor difference in 
installation in someones setup does not clash
with the intent from the website maintainers.

**How to update?**
From root of the docs:
 cp package.json ci/package.json.ci
 cp pnpm-lock.yaml ci/pnpm-lock.yaml.ci
