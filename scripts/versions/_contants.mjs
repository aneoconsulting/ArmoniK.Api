export const csharpPattern = /<PackageVersion>(?<version>.*)<\/PackageVersion>/;
export const csharpFiles = [
  "packages/csharp/ArmoniK.Api.Common.Channel/ArmoniK.Api.Common.Channel.csproj",
  "packages/csharp/ArmoniK.Api.Client/ArmoniK.Api.Client.csproj",
  "packages/csharp/ArmoniK.Api.Common/ArmoniK.Api.Common.csproj",
  "packages/csharp/ArmoniK.Api.Core/ArmoniK.Api.Core.csproj",
  "packages/csharp/ArmoniK.Api.Worker/ArmoniK.Api.Worker.csproj",
];

export const pythonPattern = /version = "(?<version>.*)"/g;
export const pythonFiles = ["packages/python/pyproject.toml"];

export const jsPattern = /"version": "(?<version>.*)"/g;
export const jsFiles = ["package.json"];
