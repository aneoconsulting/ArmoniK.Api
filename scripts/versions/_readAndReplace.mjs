import { resolve } from "pathe";
import consola from "consola";
import fs from "node:fs";

export function _readAndReplace(pattern, replace) {
  return (file) => {
    const data = fs.readFileSync(resolve(file), "utf8");

    const result = data.replace(pattern, replace);

    fs.writeFileSync(resolve(file), result, "utf8");

    consola.success(`Update ${file.split("/").pop()}`);
  };
}
