# Angular Example

This is a demo to explain the use of ArmoniK Api for Angular.

Please, refer to the documentation to get more information about the usage of the library. We write a special guide to help you to use ArmoniK Api with Angular.

## How to run

_You must have an ArmoniK instance running to run this example._

To run this example, you need to install the dependencies first:

```bash
pnpm install
```

Then, create a `src/proxy.conf.json` from the `src/proxy.conf.json.example` file. You can do it by running:

```bash
cp src/proxy.conf.json.example src/proxy.conf.json
```

And replace `<ip>:<port>` with the IP and port of your ArmoniK instance.

Finally, you can run the example:

```bash
pnpm start
```
