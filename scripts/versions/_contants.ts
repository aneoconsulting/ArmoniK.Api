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
// The Rust crates form a Cargo workspace and take their version from `[workspace.package]`; each member
// declares `version.workspace = true` and so carries no version of its own. Pointing this at a member
// instead would silently match nothing, leaving `update-versions` to skip Rust entirely and
// `verify-versions` with one fewer version to compare — a release that looks clean with a stale crate
// version.
export const rustFiles = ['packages/rust/Cargo.toml']

// `armonik` depends on `armonik-transport` by path *and* by version, because a path dependency cannot be
// published: `cargo publish` rewrites it into that version requirement, so it has to name the version
// being released or the published crate points at the wrong one.
//
// Matched through lookaround, so only the version text is replaced: anchoring on the dependency name is
// what keeps this from touching any of the third-party versions in the same file, and replacing just the
// version keeps the rest of the dependency spec (`optional`, features, ...) out of the script entirely.
// It lives in a different file from `rustFiles` on purpose — `verify-versions` keys its findings by
// file, so a second pattern on `packages/rust/Cargo.toml` would silently overwrite the first instead of
// checking both.
export const rustDependencyPattern
  = /(?<=^armonik-transport = \{ path = "\.\.\/armonik-transport", version = ")(?<version>.*?)(?:-beta-\d+)?(?=")/m
export const rustDependencyFiles = ['packages/rust/armonik/Cargo.toml']
