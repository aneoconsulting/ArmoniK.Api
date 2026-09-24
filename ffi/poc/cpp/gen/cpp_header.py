"""RETIRED (FIX-PLAN WP5 step 2). A forwarding name only.

The C header of ABI v1 is rendered by the SHARED C++ backend, `poc/codec/gen/cpp_abi.py`,
from the plan (`plan.py`'s layout functions, `plan.rpc`, `plan.lifecycle`). The java and
python slices' generators import `cpp_header.emit` from this directory read-only; this
module keeps that name working while they port (WP5 steps 3 and 5). It decides nothing.
Delete it when no generator imports `cpp_header`.
"""
from cpp_abi import emit  # noqa: F401
