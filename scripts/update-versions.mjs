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
csharpFiles.forEach(
  _readAndReplace(csharpPattern, `<PackageVersion>${version}</PackageVersion>`)
);

const pythonPattern = /version = "(.*)"/g;
const pythonFiles = ["packages/python/pyproject.toml"];

consola.info("Updating Python projects to version", version);
pythonFiles.forEach(_readAndReplace(pythonPattern, `version = "${version}"`));

const jsPattern = /"version": "(.*)"/g;
const jsFiles = ["package.json"];

consola.info("Updating JS projects to version", version);
jsFiles.forEach(_readAndReplace(jsPattern, `"version": "${version}"`));

function _readAndReplace(pattern, replace) {
  return (file) => {
    const data = fs.readFileSync(resolve(file), "utf8");

    const result = data.replace(pattern, replace);

    fs.writeFileSync(resolve(file), result, "utf8");

    consola.success(`Update ${file.split("/").pop()}`);
  };
}
