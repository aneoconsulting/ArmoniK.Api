import fs from 'node:fs'
import process from 'node:process'
import { resolve } from 'pathe'
import consola from 'consola'

export function _readAndFind(pattern: RegExp, versions: Map<string, string>, key?: string) {
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

    versions.set(key ?? file, version)
    consola.log(`Found ${key ?? file.split('/').pop()}@${version}`)
  }
}
