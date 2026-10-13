# Web package

Generated from the `.proto` files using [ts-proto](https://www.npmjs.com/package/ts-proto): TypeScript services and
messages, with no bundled gRPC client or server implementation — use it as the typed building block for a custom
client or app. If you're building an Angular app specifically, use the [Angular package](angular.md) instead.

## Installation

```bash
npm install @aneoconsultingfr/armonik.api
```

## Updating

The web package is published to npm automatically by CI on every release.

```{note}
There is an edge release on every commit to `main`, published under the `next` tag.
```

## Using it

There's no dedicated quickstart for the raw web package; see [Use ArmoniK API in an Angular App](../how-to/angular-integration.md)
for how the generated services are typically consumed, adapting the transport setup to your own framework if not
using Angular.
