import fs from 'node:fs'
import readline from 'node:readline'
import process from 'node:process'
import consola from 'consola'

const [, , ...args] = process.argv

if (args.length === 0) {
  consola.error('Please provide a file path to read and write')
  consola.log('Example: node remove-useless-md.mjs <src_file> <dst_file>')
  process.exit(1)
}

const srcFile = args[0]
if (!srcFile) {
  consola.error('Please provide a file path to read')
  process.exit(1)
}
const dstFile = args[1]
if (!dstFile) {
  consola.error('Please provide a file path to write')
  process.exit(1)
}

consola.info(`Read file ${srcFile}`)
const file = readline.createInterface({
  input: fs.createReadStream(srcFile),
  crlfDelay: Number.POSITIVE_INFINITY
})

// Write to file
consola.info(`Create write stream to ${dstFile}`)
const writeStream = fs.createWriteStream(dstFile)

const useLessLines = []
const tocHeader = '## Table of Contents'
let isInToc = false

consola.info('Start to remove useless lines')
file.on('line', (line) => {
  if (line.includes('# Protocol Documentation')) {
    line = line.replace('# Protocol Documentation', '# V1')
  }

  if (line.includes('##')) {
    isInToc = false
  }

  if (line.includes(tocHeader)) {
    isInToc = true
  }

  if (isInToc) {
    return
  }

  if (useLessLines.some(useLessLine => line.includes(useLessLine))) {
    return
  }

  if (line.includes('<a ')) {
    // Replace name to id
    line = line.replace(/name="(.*)"/, 'id="$1"')
  }

  writeStream.write(`${line}\r`)
})

file.on('close', () => {
  consola.info('Done')
})
