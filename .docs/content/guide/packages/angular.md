---
navigation.icon: vscode-icons:file-type-angular
---

# Angular package

The Angular package is build on top of [protoc-gen-ng package](https://www.npmjs.com/package/@ngx-grpc/protoc-gen-ng). This tool is used to generate Angular services and messages from `.proto` files.

## Update Angular Package

Nothing to do here. The Angular package is automatically updated by the CI/CD pipeline when a new release is published.

```{note}

There is an edge release on every commit on the `main` branch. You can use it to test the latest features using the `next` tag of the package.
```

## Manual update

To add export to the Angular package, you need to update the `index.ts` file in the projects `aneoconsultingfr/armonik.api.angular` folder.

Before that, you need to generate the proto files using the `protoc` command from the root for the angular package.

```bash
npm run proto:generate:linux
```

```{warning}
The `protoc` command is only available on Linux.
```

Then, you can update the `index.ts` file.
