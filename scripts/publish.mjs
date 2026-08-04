// @ts-nocheck
import process from 'node:process'
import { $ } from 'zx'
import consola from 'consola'

// Usage: zx scripts/publish.mjs <package> <distFolder> <distTag> [--dry-run]
// `distFolder` is `.` for packages published straight from their own directory.
// `--dry-run` builds the tarball and reports what would be published, without
// contacting the registry.
const [packageName, distFolder, distTag, ...flags] = process.argv.slice(3)
const dryRun = flags.includes('--dry-run')

if (!packageName || !distFolder || !distTag || flags.some(flag => flag !== '--dry-run')) {
  consola.fatal('Usage: nr ci:publish <package> <distFolder> <distTag> [--dry-run]')
  process.exit(1)
}

consola.log(dryRun
  ? `Dry run: packaging ${packageName} for the ${distTag} tag without publishing...`
  : `Publishing ${packageName} under the ${distTag} tag...`)

await $`cd packages/${packageName}/${distFolder} && pnpm publish --access public --no-git-checks --tag ${distTag} ${dryRun ? ['--dry-run'] : []}`
