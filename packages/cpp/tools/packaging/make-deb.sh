#!/usr/bin/env bash
set -x
script_path="$(readlink -f "${BASH_SOURCE:-$0}")"
script_dir="$(dirname "$script_path")"
docker build -t "debbuild" -f deb.Dockerfile --progress=plain "${script_dir}/../../../.."
