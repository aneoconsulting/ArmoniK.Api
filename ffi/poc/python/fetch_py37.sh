#!/usr/bin/env bash
# The 3.7 floor (owner D1), obtained without the deadsnakes PPA or python.org (both refused
# by this container's egress policy): Ubuntu 18.04's own CPython 3.7.5 packages, from
# archive.ubuntu.com's pool, which the proxy serves. Extracted, not installed: nothing
# outside poc/python/build/py37 is touched.
#
#   ./fetch_py37.sh          -> build/py37/python3.7  (a wrapper setting PYTHONHOME)
#
# The binaries are bionic's (glibc 2.27), which run on a newer glibc. bionic's libffi6 is
# fetched too (`ctypes` needs it, and protobuf 4.24's import chain reaches ctypes); libssl1.1
# is not, so `ssl` and hashlib's OpenSSL algorithms are unavailable. The gates need neither.
#
# The incumbent for 3.7 (the conformance gate's upb arm) is protobuf 4.24.4 (cp37-abi3) and
# grpcio-tools 1.59.3, downloaded from PyPI into build/py37/site; shapes_pb2 for that
# runtime is generated into build/py3.7/pb2, which arms.py prefers on 3.7.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
D="$HERE/build/py37"
mkdir -p "$D/debs" "$D/root"
BASE=http://archive.ubuntu.com/ubuntu/pool/universe/p/python3.7
V=3.7.5-2ubuntu1~18.04.2
FFI6=http://archive.ubuntu.com/ubuntu/pool/main/libf/libffi/libffi6_3.2.1-8_amd64.deb
FFI6_SHA=fa26945b0aadfc72ec623c68be9cc59235a7fe42e2388f7015fd131f4fb06dc9
declare -A SHA=(
  [python3.7-minimal]=d331e69b33a9f7eebc19ae748bae849b88345ac2b2dab7fd45144b6dab7b71f0
  [libpython3.7-minimal]=c069fe4c3f03b9e02b85e44ae3c1c6245c4fea165ef3c6dd636039c199d6c767
  [libpython3.7-stdlib]=d47096aafdcc49d51eb42d6ec86e698fb3498702150ac323d000598e881df86a
  [libpython3.7-dev]=bb1a3b7b47b203570518a83656f8c9e2ee33846efdaefa3dfe7e55c5f122b301
  [libpython3.7]=c88032bc1380ddc50f5bffbc354dd781e45855408c923c7332495f515f821c36
  [python3.7]=b4b115f8c34abedd615738ada98f5ba79cf9a6b9d819c5a48c4aa01dce7cf57a
)
for p in "${!SHA[@]}"; do
  f="$D/debs/${p}_${V}_amd64.deb"
  [ -f "$f" ] || curl -sS -o "$f" "$BASE/${p}_${V}_amd64.deb"
  echo "${SHA[$p]}  $f" | sha256sum -c --quiet - || { echo "sha256 mismatch: $f"; exit 1; }
  dpkg-deb -x "$f" "$D/root"
done
f="$D/debs/$(basename "$FFI6")"
[ -f "$f" ] || curl -sS -o "$f" "$FFI6"
echo "$FFI6_SHA  $f" | sha256sum -c --quiet - || { echo "sha256 mismatch: $f"; exit 1; }
dpkg-deb -x "$f" "$D/root"
cat > "$D/python3.7" <<EOF
#!/usr/bin/env bash
export PYTHONHOME="$D/root/usr"
export LD_LIBRARY_PATH="$D/root/usr/lib/x86_64-linux-gnu\${LD_LIBRARY_PATH:+:\$LD_LIBRARY_PATH}"
export PYTHONPATH="$D/site\${PYTHONPATH:+:\$PYTHONPATH}"
exec "$D/root/usr/bin/python3.7" "\$@"
EOF
chmod +x "$D/python3.7"
# The incumbent for the conformance gate, from PyPI (reachable), for cp37.
mkdir -p "$D/wheels"
python3.12 -m pip download -q --only-binary=:all: --python-version 3.7 --platform manylinux2014_x86_64 \
  --implementation cp --abi cp37m --abi abi3 --abi none "protobuf==4.24.4" "grpcio-tools==1.59.3" -d "$D/wheels"
python3.12 -m pip install -q --no-deps --target "$D/site" --platform manylinux2014_x86_64 --implementation cp \
  --python-version 3.7 --only-binary=:all: "$D"/wheels/*.whl 2>/dev/null
mkdir -p "$HERE/build/py3.7/pb2"
(cd "$HERE/../../schema/generated" && "$D/python3.7" -W ignore -m grpc_tools.protoc -I. --python_out="$HERE/build/py3.7/pb2" shapes.proto)
"$D/python3.7" -c 'import sys, sysconfig, google.protobuf as p; from google.protobuf.internal import api_implementation as a; print(sys.version.split()[0], sysconfig.get_config_var("EXT_SUFFIX"), sysconfig.get_paths()["include"], "protobuf", p.__version__, a.Type())'
