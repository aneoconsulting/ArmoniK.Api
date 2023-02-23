import { execSync } from 'node:child_process'
import { bumpVersion } from 'changelogen'

function main() {
  const branch = execSync('git rev-parse --abbrev-ref HEAD').toString('utf-8').trim();
  const currentCommit = execSync('git rev-parse HEAD').toString('utf-8').trim();
  const latestTagCommit = execSync('git rev-list --tags --max-count=1').toString('utf-8').trim();
  const version = execSync('git describe --abbrev=0 --tags').toString('utf-8').trim();

  const bumpedVersion = bumpVersion([latestTagCommit, currentCommit], {

  });

  console.log(version);
}

try {
  main()
} catch (error) {
  console.error(error);
  process.exit(1);
}
