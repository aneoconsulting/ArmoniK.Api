---
navigation.icon: vscode-icons:file-type-typescript
---

# Web package

The TypeScript package is build on top of [ts-proto](https://www.npmjs.com/package/ts-proto). This tool is used to generate TypeScript services and messages from `.proto` files.

This package does not include any gRPC client or server implementation. You can use it to build a custom client or app.

## Update Web Package

Nothing to do here. The TypeScript package is automatically updated by the CI/CD pipeline when a new release is published.

```{note}

There is an edge release on every commit on the `main` branch. You can use it to test the latest features using the `next` tag of the package.
```
