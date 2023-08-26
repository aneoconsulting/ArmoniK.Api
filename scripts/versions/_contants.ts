import {glob, globSync} from "glob"

export const csharpPattern = /<PackageVersion>(?<version>.*)<\/PackageVersion>/
export const csharpFiles = glob.globSync(`**/*.csproj`)

export const pythonPattern = /version = "(?<version>.*)"/g
export const pythonFiles = ['packages/python/pyproject.toml']

export const jsPattern = /"version": "(?<version>.*)"/
export const jsFiles = ['packages/angular/projects/aneoconsultingfr/armonik.api.angular/package.json', 'packages/web/package.json']

export const cppPattern = /set\(version (?<version>.*)\)/
export const cppFiles = ['packages/cpp/CMakeLists.txt']
