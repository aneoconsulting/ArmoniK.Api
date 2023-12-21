import process from 'node:process'
import consola from 'consola'
import { _readAndReplace } from './versions/_readAndReplace'
import {
  cppFiles,
  cppPattern,
  csharpFiles,
  csharpPatternPackageVersion,
  csharpPatternVersion,
  jsFiles,
  jsPattern,
} from './versions/_contants'

const [, , ...args] = process.argv

if (args.length === 0) {
  consola.fatal('Please provide a version')
  consola.log('Usage: npm run update-versions <version>')
  consola.log('Example: npm run update-versions 1.0.0')
  process.exit(1)
}

const version = args[0]

consola.info('Updating C# <PackageVersion> projects to ', version)
csharpFiles.forEach(
  _readAndReplace(csharpPatternPackageVersion, `<PackageVersion>${version}</PackageVersion>`),
)
consola.info('Updating C# <Version> projects to ', version)
csharpFiles.forEach(
  _readAndReplace(csharpPatternVersion, `<Version>${version}</Version>`),
)

consola.info('Updating JS projects to ', version)
jsFiles.forEach(_readAndReplace(jsPattern, `"version": "${version}"`))

consola.info('Updating cpp projects to ', version)
cppFiles.forEach(_readAndReplace(cppPattern, `set(version ${version})`))
