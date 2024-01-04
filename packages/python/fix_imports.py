#!/usr/bin/env python3
# Fixed version of fix_protobuf_imports available here under MIT License : https://pypi.org/project/fix-protobuf-imports/
import os
import re
import sys
from typing import Tuple
from pathlib import Path

if sys.version_info >= (3, 8):
    from typing import TypedDict  # pylint: disable=no-name-in-module
else:
    from typing_extensions import TypedDict

import argparse

class ProtobufFilePathInfo(TypedDict):
    dir: Path
    path: Path
    rel_path: Path

def fix_protobuf_imports(root_dir, dry):
    """
      A script to fix relative imports (from and to nested sub-directories) within compiled `*_pb2*.py` Protobuf files.
    """

    root_dir = Path(root_dir)

    def generate_lookup(path: Path) -> Tuple[str, ProtobufFilePathInfo]:
        name = path.name.split(".")[0]
        rel_path = path.relative_to(root_dir)
        directory = path.parent.relative_to(root_dir)

        return (name, {"dir": directory, "path": path, "rel_path": rel_path})

    py_files = list(root_dir.glob("**/*_pb2*.py"))
    pyi_files = list(root_dir.glob("**/*_pb2*.pyi"))

    py_files_dictionary = {}
    for path in py_files:
        name, info = generate_lookup(path)
        py_files_dictionary[name] = info

    pyi_files_dictionary = {}
    for path in pyi_files:
        name, info = generate_lookup(path)
        pyi_files_dictionary[name] = info

    def fix_protobuf_import_in_line(
        original_line, referencing_info: ProtobufFilePathInfo, pyi=False
    ) -> str:
        line = original_line

        m = re.search(r"^import\s(\S*_pb2)\sas\s(.*)$", line)

        if m is not None:
            referenced_name = m.group(1)
            referenced_alias = m.group(2)

            referenced_directory = py_files_dictionary[referenced_name]["dir"]
            relative_path_to_referenced_module = os.path.relpath(
                referenced_directory, referencing_info["dir"]
            )

            uppath_levels = relative_path_to_referenced_module.count("..")

            original_line = line.replace("\n", "")

            downpath = (
                relative_path_to_referenced_module.split("..")[-1]
                .replace("/", ".")
                .replace("\\", ".")
            )
            if referenced_alias:
                line = f'from .{"." * uppath_levels}{downpath if downpath != "." else ""} import {referenced_name} as {referenced_alias}\n'.replace(
                    "from ...", "from ..")
            else:
                line = f'from .{"." * uppath_levels}{downpath if downpath != "." else ""} import {referenced_name}\n'.replace(
                    "from ...", "from ..")

            new_line = line.replace("\n", "")

            print(f'{referencing_info["rel_path"]}: "{original_line}" -> "{new_line}"')
        else:
            m = re.search(r"^from\s([^\s\.]+[\S]*)\simport\s(.*_pb2)$", line)

            if m is not None:
                import_path = m.group(1).replace(".", "/")

                referenced_directory = root_dir / import_path

                if referenced_directory.exists():
                    relative_path_to_root = os.path.relpath(
                        root_dir, referencing_info["dir"]
                    )

                    uppath_levels = relative_path_to_root.count("..")

                    original_line = line.replace("\n", "")

                    line = (
                        f'from .{"." * uppath_levels}{m.group(1)} import {m.group(2)}\n'
                    ).replace("from ...", "from ..")

                    new_line = line.replace("\n", "")

                    print(
                        f'{referencing_info["rel_path"]}: "{original_line}" -> "{new_line}"'
                    )

        return line

    def fix_protobuf_imports_in_file(name, info: ProtobufFilePathInfo, pyi=False):
        with open(info["path"], "r+" if not dry else "r") as f:
            lines = f.readlines()
            if not dry:
                f.seek(0)

            for line in lines:
                line = fix_protobuf_import_in_line(line, info, pyi)

                if not dry:
                    f.writelines([line])

            if not dry:
                f.truncate()
            f.close()

    for (name, info) in py_files_dictionary.items():
        fix_protobuf_imports_in_file(name, info)

    for (
        name,
        info,
    ) in pyi_files_dictionary.items():
        fix_protobuf_imports_in_file(name, info, pyi=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("root_dir", type=Path, help="Path to the root directory")
    parser.add_argument("--dry", action="store_true", default=False, help="Do not write out the changes to the files.")
    args = parser.parse_args()
    if not args.root_dir.is_dir():
        raise argparse.ArgumentTypeError(f"Directory '{args.root_dir}' does not exist.")
    fix_protobuf_imports(args.root_dir, args.dry)


if __name__ == '__main__':
    sys.argv[0] = re.sub(r'(-script\.pyw|\.exe)?$', '', sys.argv[0])
    exit(main())
