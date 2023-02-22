import { resolve } from "pathe";
import consola from "consola";
import { promises as fsp } from "node:fs";

export function _readAndFind(pattern: RegExp, versions: Map<string, string>) {
  return async (file: string) => {
    const data = await fsp.readFile(resolve(file), "utf8");

    const version = pattern.exec(data)?.groups?.version;

    if (!version) {
      consola.fatal(`Could not find version in ${file}`);
      process.exit(1);
    }

    versions.set(file, version);
    consola.trace(`Found ${file.split("/").pop()}@${version}`);
  };
}
