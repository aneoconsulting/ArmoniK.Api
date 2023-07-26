import { $ } from "zx"
import { inc } from "semver"
import { execSync } from "node:child_process"
import { promises as fsp } from "node:fs"
import { readPackageJSON, writePackageJSON } from "pkg-types"
import { loadChangelogConfig, parseCommits, getGitDiff, determineSemverChange } from "changelogen"

export async function determineBumpType() {
  const config = await loadChangelogConfig(process.cwd())
  const latestTag = execSync('git describe --tags --abbrev=0').toString('utf-8').trim()

  const commits = parseCommits(await getGitDiff(latestTag), config)
  const bumpType = determineSemverChange(commits, config)

  return bumpType === 'major' ? 'minor' : bumpType
}

async function main() {
  const [tag] = process.argv.slice(3)

  const packageJson = JSON.parse(await (await fsp.readFile("packages/angular/projects/aneoconsultingfr/armonik.api.angular/package.json")).toString('utf-8').trim())

  const bumpType = await determineBumpType(packageJson.version, tag)

  const commit = execSync('git rev-parse --short HEAD').toString('utf-8').trim().slice(0, 8)
  const date = Math.round(Date.now() / (1000 * 60))
  const newVersion = inc(packageJson.version, bumpType || 'patch', tag)

  packageJson.version = `${newVersion}-${date}-${commit}`
  await writePackageJSON("packages/angular/projects/aneoconsultingfr/armonik.api.angular/package.json", packageJson)

  await $`cd packages/angular/dist/aneoconsultingfr/armonik.api.angular && pnpm publish --access public --no-git-checks --tag ${tag}`
}

main().catch((err) => {
  console.error(err)
  process.exit(1)
})
