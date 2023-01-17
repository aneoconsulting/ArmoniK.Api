import consola from "consola";
import {
  csharpFiles,
  csharpPattern,
  jsFiles,
  jsPattern,
  pythonFiles,
  pythonPattern,
} from "./versions/_contants.mjs";
import { _readAndFind } from "./versions/_readAndFind.mjs";

const versions = [];

consola.info("Finding C# projects versions");
csharpFiles.forEach(_readAndFind(csharpPattern, versions));
consola.info("Finding Python projects versions");
pythonFiles.forEach(_readAndFind(pythonPattern, versions));
consola.info("Finding JS projects versions");
jsFiles.forEach(_readAndFind(jsPattern, versions));

const uniqueVersions = [...new Set(versions)];

if (uniqueVersions.length > 1) {
  consola.fatal("Found multiple versions", uniqueVersions);
  process.exit(1);
}

consola.success("Found version", uniqueVersions[0]);
