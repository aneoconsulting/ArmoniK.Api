#!/bin/sh

set -e

SOLUTION_FILE=$(realpath ./packages/csharp/ArmoniK.Api.sln)
OUTPUT_DIR=.docs/content/how-to/envars

dotnet tool install -g ArmoniK.Utils.DocExtractor

cd $OUTPUT_DIR
armonik.utils.docextractor -s $SOLUTION_FILE
