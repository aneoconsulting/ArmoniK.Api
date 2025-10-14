#!/bin/sh

set -e

SOLUTION_FILE=$(realpath ./packages/csharp/ArmoniK.Api.sln)
OUTPUT_DIR=.docs/content/usage/envars

dotnet tool install -g ArmoniK.Utils.DocExtractor --version 0.6.2

cd $OUTPUT_DIR
armonik.utils.docextractor -s $SOLUTION_FILE
