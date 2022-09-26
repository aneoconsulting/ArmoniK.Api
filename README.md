# ArmoniK.Api

This project is part of the [ArmoniK](https://github.com/aneoconsulting/ArmoniK) project.
In particular it is a consitutent of its [Core](https://github.com/aneoconsulting/ArmoniK.Core)
component which, among its main functionalities, implements several gRPC services aiming to
provide a user with a robust task scheduler for a high throughput computing application.

The .proto files in the directory [./Protos/V1](https://github.com/aneoconsulting/ArmoniK.Api/tree/main/Protos/V1) 
provide the core data model and functionalities for ArmoniK and are used to generate the different SDKs.

## Documentation

[See this link](https://aneoconsulting.github.io/ArmoniK.Api/api/index.html) to explore documentation.

Documentation for `.proto` files is generated with the plugin [protoc-gen-doc](https://github.com/pseudomuto/protoc-gen-doc) during the build process in the CI/CD pipeline.

Documentation for `.csproj` files is generated with [DocFX](https://dotnet.github.io/docfx/) during the build process in the CI/CD pipeline.

## Contributing

Contributions are always welcome!

See [CONTRIBUTING.md](https://github.com/aneoconsulting/ArmoniK.Api/blob/main/CONTRIBUTING.md) for ways to get started.

### Improve Protocol Documentation

Please refer to [protoc-gen-doc](https://github.com/pseudomuto/protoc-gen-doc) to make sure to adhere to
the format the plugin expects to correctly generate documentation. You can find examples at the end section of [protoc-gen-doc](https://github.com/pseudomuto/protoc-gen-doc#output-example).
