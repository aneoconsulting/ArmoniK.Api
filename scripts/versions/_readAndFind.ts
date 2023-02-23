import { resolve } from "pathe";
import consola from "consola";
import fs from "node:fs";

export function _readAndFind(pattern: RegExp, versions: Map<string, string>) {
  return (file: string) => {
    const data = fs.readFileSync(resolve(file), {
      encoding: "utf8",
      flag: "r",
    });

    const version = pattern.exec(data)?.groups?.version;

    if (!version) {
      consola.fatal(`Could not find version in ${file}`);
      process.exit(1);
    }

    versions.set(file, version);
    consola.log(`Found ${file.split("/").pop()}@${version}`);

  };
}
