// Read file line by line
import fs from "node:fs";
import readline from "node:readline";

const [, , ...args] = process.argv;

// TODO: Update to consola when a package.json is added
if (args.length === 0) {
  console.error("Please provide a file path to read and write");
  console.error("Example: node remove-useless-md.mjs <src_file> <dst_file>");
  process.exit(1);
}

const srcFile = args[0];
if (!srcFile) {
  console.error("Please provide a file path to read");
  process.exit(1);
}
const dstFile = args[1];
if (!dstFile) {
  console.error("Please provide a file path to write");
  process.exit(1);
}

console.info(`Read file ${srcFile}`);
const file = readline.createInterface({
  input: fs.createReadStream(srcFile),
  crlfDelay: Infinity,
});

// Write to file
console.info(`Create write stream to ${dstFile}`);
const writeStream = fs.createWriteStream(dstFile);

const useLessLines = [];
const tocHeader = "## Table of Contents";
let isInToc = false;

console.info("Start to remove useless lines");
file.on("line", (line) => {
  if (line.includes("##")) {
    isInToc = false;
  }

  if (line.includes(tocHeader)) {
    isInToc = true;
  }

  if (isInToc) return;

  if (useLessLines.some((useLessLine) => line.includes(useLessLine))) {
    return;
  }

  if (line.includes("<a ")) {
    // Replace name to id
    line = line.replace(/name="(.*)"/, `id="$1"`);
  }

  writeStream.write(line + "\r");
});

file.on("close", () => {
  console.info("Done");
});
