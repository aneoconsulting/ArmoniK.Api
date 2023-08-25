import fs from 'node:fs'
import { resolve } from 'pathe'
import consola from 'consola'

export function _readAndReplace(pattern: RegExp, replace: string) {
  return (file: string) => {
    const data = fs.readFileSync(resolve(file), 'utf8')

    const result = data.replace(pattern, replace)

    fs.writeFileSync(resolve(file), result, 'utf8')

    consola.success(`Update ${file.split('/').pop()}`)
  }
}
