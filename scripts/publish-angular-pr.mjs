import { $ } from "xz"
import { inc } from "semver"

const [tag] = process.argv.slice(3)

const newVersion = ""
// TODO: read package.json
// TODO: update package.json version

await $`cd packages/angular/dist/aneoconsultingfr/armonik.api.angular && pnpm publish --access public --no-git-checks --tag ${tag}`
