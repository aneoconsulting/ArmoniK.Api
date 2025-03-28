---
navigation.icon: heroicons:newspaper
---

<!-- @case-police-ignore Api -->

# Releases

A release is created when there is enough new features or bug fixes to justify a new version. A release is created from the `main` branch and is tagged with the version number following [Semantic Versioning](https://semver.org/).

## Create a release

In order to be sure that every [packages](./packages) use the same version, we create script to automate the process. This script is written using [NodeJS](https://nodejs.org/en/) and can be found in the `scripts` folder.

### Prerequisites

- [NodeJS](https://nodejs.org/en/) (latest LTS version)

### Steps

- Install dependencies

```bash
pnpm install
```

```{note}

You can install pnpm using `npm i -g pnpm`

```
- Run the script

```bash
pnpm run update-versions <version>
```

- Create a PR with the changes (an action will be triggered to check that each package has the same version)
- Merge the PR
- Create a new release on GitHub

And that's it! The new version is now available on all packages.

## Edge Release Channel

API is landing commits, improvements and bug fixes every day. You can opt-in to the Edge release channel to get the latest features and fixes as soon as they are ready.

After each commit is merged into the `main` branch, packages are built and deployed to registries.

The build and publishing method and quality of edge releases are the same as stable ones. The only difference is that you should often check the GitHub repository for updates. There is a slight change of regressions not being caught during the review process and by the automated tests. Therefore, we internally use this channel to double-check everything before each release.

### C# packages

C# packages are available on [NuGet](https://www.nuget.org).

- [ArmoniK.Api.Client](https://www.nuget.org/packages/ArmoniK.Api.Client/)
- [ArmoniK.Api.Common](https://www.nuget.org/packages/ArmoniK.Api.Common/)
- [ArmoniK.Api.Common.Channel](https://www.nuget.org/packages/ArmoniK.Api.Common.Channel/)
- [ArmoniK.Api.Core](https://www.nuget.org/packages/ArmoniK.Api.Core/)
- [ArmoniK.Api.Worker](https://www.nuget.org/packages/ArmoniK.Api.Worker/)


C# generate also packages on each PR commit. This is useful to test or implement features in parallel to validate that proto are correct. You can find the latest version on [NuGet](https://www.nuget.org/profiles/ANEO).

```{note}

C# packages are available on Edge channel.


```
### Python

Python package is available on [PyPi](https://pypi.org/).

- [armonik](https://pypi.org/project/armonik/)

```{warning}
Python package is not yet available on Edge channel.

```

### Angular

Angular package is available on [NPM](https://www.npmjs.com).

- [@aneoconsultingfr/armonik.api](https://www.npmjs.com/package/@aneoconsultingfr/armonik.api.angular)

```{note}

Angular package is available on Edge channel.


```
### Web

Web package is available on [NPM](https://www.npmjs.com).

- [@aneoconsultingfr/armonik.api](https://www.npmjs.com/package/@aneoconsultingfr/armonik.api)

```{note}

Web package is available on Edge channel.
```
