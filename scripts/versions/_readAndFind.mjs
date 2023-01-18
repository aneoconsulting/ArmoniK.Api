import { resolve } from "pathe";
import consola from "consola";
import fs from "node:fs";

export function _readAndFind(pattern, versions) {
  return (file) => {
    const data = fs.readFileSync(resolve(file), "utf8");

    const version = pattern.exec(data).groups?.version;

    versions.push(version);

    consola.log(`Found ${file.split("/").pop()}@${version}`);
  };
}
