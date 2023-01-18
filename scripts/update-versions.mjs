import consola from "consola";
import { _readAndReplace } from "./versions/_readAndReplace.mjs";
import {
  csharpFiles,
  csharpPattern,
  jsFiles,
  jsPattern,
  pythonFiles,
  pythonPattern,
} from "./versions/_contants.mjs";

const [, , ...args] = process.argv;

if (args.length === 0) {
  consola.fatal("Please provide a version");
  process.exit(1);
}

const version = args[0];

consola.info("Updating C# projects to version", version);
csharpFiles.forEach(
  _readAndReplace(csharpPattern, `<PackageVersion>${version}</PackageVersion>`)
);

consola.info("Updating Python projects to version", version);
pythonFiles.forEach(_readAndReplace(pythonPattern, `version = "${version}"`));

consola.info("Updating JS projects to version", version);
jsFiles.forEach(_readAndReplace(jsPattern, `"version": "${version}"`));
