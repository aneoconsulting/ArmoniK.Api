# Angular package

Generated from the `.proto` files using [protoc-gen-ng](https://www.npmjs.com/package/@ngx-grpc/protoc-gen-ng):
Angular services and messages, including a gRPC-web client implementation ready to inject into an app.

## Installation

```bash
npm install @aneoconsultingfr/armonik.api.angular
```

## Updating

The Angular package is published to npm automatically by CI on every release.

```{note}
There is an edge release on every commit to `main`, published under the `next` tag.
```

## Using it

See [Use ArmoniK API in an Angular App](../how-to/angular-integration.md) for a worked, tested example (service
injection, calling `ListPartitions`, and the gRPC-web gateway errors you'll hit along the way).

## Maintainer note: adding a new export

To add a new export to the Angular package, update the `index.ts` file in the
`projects/aneoconsultingfr/armonik.api.angular` folder. Before that, generate the proto files for the Angular
package from the repository root:

```bash
npm run proto:generate:linux
```

```{warning}
The `protoc` command is only available on Linux.
```

Then update the `index.ts` file.
