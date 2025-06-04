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
export const javaFiles = ['packages/java/pom.xml']

export const rustPattern = /^version\s*=\s*"(?<version>.*?)(?:-beta-\d+)?"$/m
export const rustFiles = ['packages/rust/armonik/Cargo.toml']
