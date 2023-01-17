import fs from "node:fs";
import consola from "consola";
import { resolve } from "pathe";

const [, , ...args] = process.argv;

if (args.length === 0) {
  consola.fatal("Please provide a version");
  process.exit(1);
}

const version = args[0];

const csharpPattern = /<PackageVersion>(.*)<\/PackageVersion>/g;
const csharpFiles = [
  "packages/csharp/ArmoniK.Api.Common.Channel/ArmoniK.Api.Common.Channel.csproj",
  "packages/csharp/ArmoniK.Api.Client/ArmoniK.Api.Client.csproj",
  "packages/csharp/ArmoniK.Api.Common/ArmoniK.Api.Common.csproj",
  "packages/csharp/ArmoniK.Api.Core/ArmoniK.Api.Core.csproj",
  "packages/csharp/ArmoniK.Api.Worker/ArmoniK.Api.Worker.csproj",
];

consola.info("Updating C# projects to version", version);
csharpFiles.forEach((file) => {
  const data = fs.readFileSync(resolve(file), "utf8");

  const result = data.replace(
    csharpPattern,
    `<PackageVersion>${version}</PackageVersion>`
  );

  fs.writeFileSync(resolve(file), result, "utf8");

  consola.success(`Updated ${file.split("/").pop()}`);
});

const pythonPattern = /version = "(.*)"/g;
const pythonFiles = ["packages/python/pyproject.toml"];
consola.info("Updating Python projects to version", version);
pythonFiles.forEach((file) => {
  const data = fs.readFileSync(resolve(file), "utf8");

  const result = data.replace(pythonPattern, `version = "${version}"`);

  fs.writeFileSync(resolve(file), result, "utf8");

  consola.success(`Updated ${file.split("/").pop()}`);
});
