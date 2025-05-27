import process from 'node:process'
import consola from 'consola'
import {
  cppFiles,
  cppPattern,
  csharpFiles,
  csharpPatternPackageVersion,
  csharpPatternVersion,
  javaFiles,
  javaPattern,
  jsFiles,
  jsPattern,
  rustFiles,
  rustPattern,
} from './versions/_contants'
import { _readAndFind } from './versions/_readAndFind'

const versions = new Map<string, string>()
const [, , ...args] = process.argv

consola.info('Finding JS projects versions')
jsFiles.forEach(_readAndFind(jsPattern, versions))
consola.info('Finding C# <PackageVersion> projects versions')
csharpFiles.forEach(_readAndFind(csharpPatternPackageVersion, versions))
consola.info('Finding C# <Version> projects versions')
csharpFiles.forEach(_readAndFind(csharpPatternVersion, versions))
consola.info('Finding Cpp projects versions')
cppFiles.forEach(_readAndFind(cppPattern, versions))
consola.info('Finding java projects versions')
javaFiles.forEach(_readAndFind(javaPattern, versions))
consola.info('Finding rust projects versions')
rustFiles.forEach(_readAndFind(rustPattern, versions))

const versionsArray = [...versions.values()]
const uniqueVersions = [...new Set(versionsArray)]

const filesPerVersion = new Map<string, string[]>()
versions.forEach((version, file) => {
  const files = filesPerVersion.get(version) || []
  files.push(file)
  filesPerVersion.set(version, files)
})

if (uniqueVersions.length > 1) {
  consola.fatal('Found multiple versions')
  uniqueVersions.forEach((version) => {
    consola.info(version, filesPerVersion.get(version))
  })
  process.exit(1)
}
else if (args.length > 0 && uniqueVersions[0] !== args[0]) {
  consola.fatal(`Found ${uniqueVersions[0]} for all projects but does not match expected ${args[0]}`)
  process.exit(1)
}
else {
  consola.success(`Found ${uniqueVersions[0]} for all projects`)
  process.exit(0)
}
