import { determineSemverChange, getGitDiff, loadChangelogConfig, parseCommits } from 'changelogen';
import consola from 'consola';
import { execSync } from 'node:child_process';
import semver from "semver";

async function main() {
  const from = execSync('git describe --abbrev=0 --tags').toString('utf-8').trim();
  const to = 'main'

  const config = await loadChangelogConfig(process.cwd(), {
    from,
    to
  })

  const rawCommits = await getGitDiff(from, to)
  const commits = parseCommits(rawCommits, config).filter(
    (c) =>
      config.types[c.type] &&
      !(c.type === "chore" && c.scope === "deps" && !c.isBreaking)
  )

  const type = determineSemverChange(commits, config) || "patch"
  const newVersion = semver.inc(from, type)

  console.log("from", from, "to", to, "type", type, "newVersion", newVersion);
}

main().catch((error) => {
  consola.fatal(error);
  process.exit(1);
})
