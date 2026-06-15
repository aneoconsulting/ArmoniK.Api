# ArmoniK.Api Docs

Docs for ArmoniK.Api, built with [Sphinx](https://www.sphinx-doc.org/).

## Prerequisites

All commands are run from the **repository root**.

| Tool | Purpose |
|---|---|
| Python 3 | Sphinx build |
| `protoc` + `protoc-gen-doc` | Proto API page |
| .NET SDK | C# API and env-var docs (`docfx`, `ArmoniK.Utils.DocExtractor`) |
| Doxygen | C++ API docs |

## Installation

Create and activate a Python virtual environment:

```bash
python -m venv .venv-doc
source .venv-doc/bin/activate
pip install -r .docs/requirements.txt
```

## Generating API content

Each section of the docs requires a generation step before Sphinx can build it. Run all steps that apply to your local setup.

### Proto API (`content/api/v1.md`)

Requires `protoc` and the [`protoc-gen-doc`](https://github.com/pseudomuto/protoc-gen-doc) plugin.

```bash
apt install -y protobuf-compiler   # or equivalent for your distro
protoc -I Protos/V1 --doc_out=.docs/content/api --doc_opt=markdown,tmp.md Protos/V1/*.proto
scripts/generate-proto-doc.sh      # post-processes tmp.md into v1.md
```

### C# API (`content/api/csharp/`)

Requires the [.NET SDK](https://dotnet.microsoft.com/) and installs `docfx` globally.

```bash
scripts/generate-csharp-doc.sh
```

### Environment variables (`content/usage/envars/`)

Requires the .NET SDK and installs `ArmoniK.Utils.DocExtractor` globally.

```bash
scripts/generate-envvars-doc.sh
```

### Python API (`content/api/python/`)

Requires the Python virtual environment to be active (see above).

```bash
sphinx-apidoc -o .docs/content/api/python packages/python/src/armonik
```

### C++ API (`content/api/cpp/doxygen/xml/`)

Requires [Doxygen](https://www.doxygen.nl/).

```bash
doxygen Doxyfile
```

## Building

Once all desired generation steps are complete, build the HTML output:

```bash
sphinx-build -M html .docs build
```

Open `build/html/index.html` in a browser to view the result.
