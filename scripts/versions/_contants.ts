import { glob } from 'glob'

export const csharpPatternPackageVersion = /<PackageVersion>(?<version>.*)<\/PackageVersion>/
export const csharpPatternVersion = /<Version>(?<version>.*)<\/Version>/
export const csharpFiles = glob.globSync('**/*.csproj')

export const pythonPattern = /version = "(?<version>.*)"/g
export const pythonFiles = ['packages/python/pyproject.toml']

export const jsPattern = /"version": "(?<version>.*)"/
export const jsFiles = ['packages/angular/projects/aneoconsultingfr/armonik.api.angular/package.json', 'packages/web/package.json']

export const cppPattern = /set\(version (?<version>.*)\)/
export const cppFiles = ['packages/cpp/CMakeLists.txt']

export const javaPattern = /<version>(?<version>.*)<\/version>/
export const javaFiles = ['packages/java/pom.xml', 'packages/java/armonik-client-api/pom.xml', 'packages/java/armonik-worker-api/pom.xml']

export const rustPattern = /^version\s*=\s*"(?<version>.*?)(?:-beta-\d+)?"$/m
// The crates take their version from `[workspace.package]` and carry none of their own, so pointing
// this at a member would match nothing and skip Rust without saying so.
export const rustFiles = ['packages/rust/Cargo.toml']

// The workspace declares `armonik-transport` by version as well as by path, because a path dependency
// cannot be published. Anchored on the dependency name so it cannot reach the third-party versions
// beside it, and matched through lookaround so only the version text is replaced.
export const rustDependencyPattern
  = /(?<=^armonik-transport = \{ path = "armonik-transport", version = ")(?<version>.*?)(?:-beta-\d+)?(?=")/m
export const rustDependencyFiles = ['packages/rust/Cargo.toml']
// Same file as `rustFiles`, so `verify-versions` has to key it separately: its map is keyed by file, and
// a second entry for that path would overwrite the first instead of being compared with it.
export const rustDependencyKey = 'packages/rust/Cargo.toml [armonik-transport]'
