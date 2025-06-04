# ArmoniK.Api Docs

Docs for ArmoniK.Api

## Installation

> Be aware to be at the root of the repository

```bash
python -m venv .venv-doc
```

Then activate the virtual environment:

```bash
source .venv-doc/bin/activate
```

And install dependencies:

```bash
pip install -r .docs/requirements.txt
```

## Usage

To build the docs locally, run the following command:

```bash

apt update
apt install -y protobuf-compiler
protoc -I Protos/V1 --doc_out=.docs/content/api --doc_opt=markdown,tmp.md Protos/V1/*.proto
scripts/generate-proto-doc.sh
scripts/generate-csharp-doc.sh
sphinx-apidoc -o .docs/content/api/python packages/python/src/armonik
sphinx-build -M html .docs build
```

Outputs can be found in `build/html/index.html`.
