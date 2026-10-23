# Releases

A release is created when there are enough new features or bug fixes to justify a new version. A release is created from the `main` branch and is tagged with the version number following [Semantic Versioning](https://semver.org/).

## Create a release

To make sure every [package](../packages/index.md) uses the same version, we maintain a script that automates the process. This script is written using [NodeJS](https://nodejs.org/en/) and can be found in the `scripts` folder.

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

- Update Cargo.lock

```
cargo check
```

- Create a PR with the changes (an action will be triggered to check that each package has the same version)
- Merge the PR
- Create a new release on GitHub

And that's it! The new version is now available on all packages.

## Edge Release Channel

ArmoniK.Api lands commits, improvements, and bug fixes every day. You can opt in to the Edge release channel to get the latest features and fixes as soon as they are ready.

After each commit is merged into the `main` branch, packages are built and deployed to registries.

The build and publishing method and quality of edge releases are the same as stable ones. The only difference is that you should check the GitHub repository often for updates. There is a slight chance of regressions not being caught during the review process and by the automated tests. Therefore, we internally use this channel to double-check everything before each release.

### C# packages

C# packages are available on [NuGet](https://www.nuget.org).

- [ArmoniK.Api.Client](https://www.nuget.org/packages/ArmoniK.Api.Client/)
- [ArmoniK.Api.Common](https://www.nuget.org/packages/ArmoniK.Api.Common/)
- [ArmoniK.Api.Common.Channel](https://www.nuget.org/packages/ArmoniK.Api.Common.Channel/)
- [ArmoniK.Api.Core](https://www.nuget.org/packages/ArmoniK.Api.Core/)
- [ArmoniK.Api.Worker](https://www.nuget.org/packages/ArmoniK.Api.Worker/)

C# packages are also generated on each PR commit. This is useful for testing or implementing features in parallel, to validate that the protos are correct. You can find the latest version on [NuGet](https://www.nuget.org/profiles/ANEO).

```{note}
C# packages are available on the Edge channel.
```

### Python

Python package is available on [PyPi](https://pypi.org/).

- [armonik](https://pypi.org/project/armonik/)

```{warning}
Python package is not yet available on the Edge channel.
```

### Angular

Angular package is available on [NPM](https://www.npmjs.com).

- [@aneoconsultingfr/armonik.api.angular](https://www.npmjs.com/package/@aneoconsultingfr/armonik.api.angular)

```{note}
Angular package is available on the Edge channel.
```

### Web

Web package is available on [NPM](https://www.npmjs.com).

- [@aneoconsultingfr/armonik.api](https://www.npmjs.com/package/@aneoconsultingfr/armonik.api)

```{note}
Web package is available on the Edge channel.
```
