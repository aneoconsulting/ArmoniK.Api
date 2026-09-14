import fs from 'node:fs'
import process from 'node:process'
import { resolve } from 'pathe'
import consola from 'consola'

export function _readAndFind(pattern: RegExp, versions: Map<string, string>, label?: string) {
  return (file: string) => {
    const data = fs.readFileSync(resolve(file), {
      encoding: 'utf8',
      flag: 'r',
    })

    const version = pattern.exec(data)?.groups?.version

    if (!version) {
      consola.fatal(`Could not find version in ${file}`)
      process.exit(1)
    }

    // The label keeps entries distinct when several patterns are matched
    // against the same file (e.g. a package version and a dependency pin).
    versions.set(label ? `${file} (${label})` : file, version)
    consola.log(`Found ${file.split('/').pop()}@${version}`)
  }
}
