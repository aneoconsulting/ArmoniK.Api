import consola from "consola";
import {
  csharpFiles,
  csharpPattern,
  jsFiles,
  jsPattern,
  pythonFiles,
  pythonPattern,
} from "./versions/_contants";
import { _readAndFind } from "./versions/_readAndFind";

const versions = new Map<string, string>();

consola.info("Finding JS projects versions");
jsFiles.forEach(_readAndFind(jsPattern, versions));
consola.info("Finding C# projects versions");
csharpFiles.forEach(_readAndFind(csharpPattern, versions));
consola.info("Finding Python projects versions");
pythonFiles.forEach(_readAndFind(pythonPattern, versions));

const versionsArray = [...versions.values()];
const uniqueVersions = [...new Set(versionsArray)];

const filesPerVersion = new Map<string, string[]>();
versions.forEach((version, file) => {
  const files = filesPerVersion.get(version) || [];
  files.push(file);
  filesPerVersion.set(version, files);
});


if (uniqueVersions.length > 1) {
  consola.fatal(`Found multiple versions`);
  uniqueVersions.forEach((version) => {
    consola.info(version, filesPerVersion.get(version));
  });
  process.exit(1);
} else {
  consola.success(`Found ${uniqueVersions[0]} for all projects`);
  process.exit(0);
}
