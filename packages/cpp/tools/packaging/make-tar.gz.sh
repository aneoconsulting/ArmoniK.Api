#!/bin/sh
set -ex
script_path="$(readlink -f "${BASH_SOURCE:-$0}")"
script_dir="$(dirname "$script_path")"
docker buildx build -t "tgzalpinebuild" -f "${script_dir}/tgz_alpine.Dockerfile" --progress=plain "${script_dir}/../../../.."
docker run --rm -v ".:/host" --entrypoint sh "tgzalpinebuild" -c "cp ./*.tar.gz /host/"

