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
export const rustFiles = ['packages/rust/Cargo.toml', 'packages/rust/armonik-macros/Cargo.toml']

// The workspace declares `armonik-transport` by version as well as by path (a path dependency cannot
// be published). Anchored on the dependency name so it cannot reach third-party versions beside it.
export const rustDependencyPattern
  = /(?<=^armonik-transport = \{ path = "armonik-transport", version = ")(?<version>.*?)(?:-beta-\d+)?(?=")/m
export const rustDependencyFiles = ['packages/rust/Cargo.toml']
export const rustDependencyKey = 'packages/rust/Cargo.toml [armonik-transport]'

// Exact-version pin of armonik-macros inside armonik's Cargo.toml.
export const rustMacrosPinPattern = /^version\s*=\s*"=(?<version>.*?)(?:-beta-\d+)?"$/m
export const rustMacrosPinFiles = ['packages/rust/armonik/Cargo.toml']
