// @ts-nocheck

import { $ } from 'zx'
import consola from 'consola'

const [packageName, distFolder] = process.argv.slice(3)

consola.log(`Publishing version ${packageName}...`)
await $`cd packages/${packageName}/${distFolder ?? ''} && pnpm publish --access public --no-git-checks`
