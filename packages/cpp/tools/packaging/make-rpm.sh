#!/usr/bin/env bash
set -x
script_path="$(readlink -f "${BASH_SOURCE:-$0}")"
script_dir="$(dirname "$script_path")"
docker build -t "rpmbuild" -f ubi7.Dockerfile --progress=plain "${script_dir}/../../../.."
docker run --rm -v ".:/host" --entrypoint bash "rpmbuild" -c "cp ./*.rpm /host/"
