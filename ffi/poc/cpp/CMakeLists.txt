# The cpp slice. Every arm of the comparison, at every standard level, in both linkages.
#
# R7: the configuration is named rather than assumed. Each executable carries its own
# -std, its own implementation level (target or floor), its own guard setting, its own
# decode policy and its own LINKAGE, and prints all five on its first line.
cmake_minimum_required(VERSION 3.16)
project(ffi_cpp_slice CXX)

set(CMAKE_CXX_EXTENSIONS OFF)
if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE Release)
endif()
# -DNDEBUG matters and was missing. Without it protobuf's own GOOGLE_DCHECK family is
# compiled into shapes.pb.cc, so the INCUMBENT carried debug assertions on its hot path
# while the core was a cargo release build at opt-level 3. AK_OPT lets the whole C++ side
# be rebuilt at -O3 so "the control is slower" can be tested against the optimisation
# level rather than asserted about it.
if(NOT AK_OPT)
  set(AK_OPT "-O2")
endif()
set(CMAKE_CXX_FLAGS_RELEASE "${AK_OPT} -g -DNDEBUG")

find_package(Protobuf REQUIRED)
find_package(Threads REQUIRED)

set(SCHEMA "${CMAKE_CURRENT_SOURCE_DIR}/../../schema/generated")
set(GEN "${CMAKE_CURRENT_BINARY_DIR}/gen")
file(MAKE_DIRECTORY ${GEN})

add_custom_command(
  OUTPUT ${GEN}/shapes.pb.cc ${GEN}/shapes.pb.h
  COMMAND ${Protobuf_PROTOC_EXECUTABLE} --cpp_out=${GEN} -I${SCHEMA} ${SCHEMA}/shapes.proto
  DEPENDS ${SCHEMA}/shapes.proto
  COMMENT "protoc shapes.proto")

add_library(shapes_pb STATIC ${GEN}/shapes.pb.cc)
target_include_directories(shapes_pb PUBLIC ${GEN} ${Protobuf_INCLUDE_DIRS})
target_link_libraries(shapes_pb PUBLIC ${Protobuf_LIBRARIES})
set_property(TARGET shapes_pb PROPERTY CXX_STANDARD 11)
set_property(TARGET shapes_pb PROPERTY POSITION_INDEPENDENT_CODE ON)

# ---- the core -----------------------------------------------------------------------
# THE shared core (README R0): `ffi/poc/codec/crates/ak-core`, the same crate the rust,
# java and csharp slices build, not a copy of it. This slice contributed
# `ak_enc_count_reverse` to it and gets java's two converting transcoders back.
#
# CARGO_TARGET_DIR is set on EVERY invocation, including the default one. Before R0 the
# default build inherited `cpp/core` as its workspace root and landed there by itself;
# now the workspace root is the shared one, so an unset target dir would put this
# slice's three builds in `poc/codec/target` -- shared with every other slice, and with
# each other, which is three feature sets fighting over one output path.
#
# Three builds, which never share a binary: the timed one, the counting one (README R5)
# and, under -DAK_RPC=ON, the one with ABI v1 section 9 linked in.
#
# AK_CORE_ROOT / AK_CORE_TGT: the core's workspace root and this slice's cargo output
# directory. Both default to the shared tree. They are overridable because a slice gate
# has to be run against a FIXED core while another agent changes the shared one: the
# gate is then built from a `git archive <commit> ffi/poc/codec` snapshot, and every log
# names the commit. A second output directory keeps cargo from rebuilding the shared
# tree's artifacts from the snapshot's paths and back.
set(AK_CORE_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../codec" CACHE PATH
    "workspace root of the shared core (ffi/poc/codec or a snapshot of it)")
set(AK_CORE_TGT "${CMAKE_CURRENT_SOURCE_DIR}/core-build" CACHE PATH
    "where this slice's CARGO_TARGET_DIRs live")
set(CORE_DIR "${AK_CORE_ROOT}/crates/ak-core")
set(CORE_TGT "${AK_CORE_TGT}")
set(CORE_OUT "${CORE_TGT}/target/release")
set(CORE_COUNT_OUT "${CORE_TGT}/target-count/release")

# R-G7 / FIX-PLAN WP5 step 2: every C ABI core this slice links is built WITH `init-guard`
# (ABI v1 section 3 as specified), so a binding that skipped `ak_init` would get
# AK_ERR_UNINITIALIZED on its first codec call instead of passing silently. The binding is
# rendered from plan.lifecycle and calls it; gen/corpus_all.sh's `noinit` control shows the
# guard is in the build.
set(AK_CORE_FEATURES "init-guard")
add_custom_target(core ALL
  COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CORE_TGT}/target
          cargo build --release --features ${AK_CORE_FEATURES} --manifest-path ${CORE_DIR}/Cargo.toml
  BYPRODUCTS ${CORE_OUT}/libak_core.so ${CORE_OUT}/libak_core.a
  COMMENT "cargo build the shared ak-core (cdylib + staticlib)")

add_custom_target(core_count ALL
  COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CORE_TGT}/target-count
          cargo build --release --features count,${AK_CORE_FEATURES} --manifest-path ${CORE_DIR}/Cargo.toml
  BYPRODUCTS ${CORE_COUNT_OUT}/libak_core.so
  COMMENT "cargo build the shared ak-core --features count")

# R-D7: the concurrency suite's PLANTED cores. ABI v1 section 6's two refused designs
# exist in the shared core as test-only features (`pad-widths`, `global-widths`, ak-rt's
# manifest). Before this, the planted C++ builds planted `ak::Enc` only and linked the
# UNPLANTED core, so the suite's ffi arm read 0 in every planted build and nothing showed
# it could fail. Correctness fixtures only; never linked into a timed or gated binary.
set(CORE_PAD_OUT    "${CORE_TGT}/target-pad/release")
set(CORE_GLOBAL_OUT "${CORE_TGT}/target-global/release")
set(CORE_BOTH_OUT   "${CORE_TGT}/target-both/release")
add_custom_target(core_pad ALL
  COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CORE_TGT}/target-pad
          cargo build --release --features pad-widths,${AK_CORE_FEATURES} --manifest-path ${CORE_DIR}/Cargo.toml
  BYPRODUCTS ${CORE_PAD_OUT}/libak_core.so
  COMMENT "cargo build the shared ak-core --features pad-widths (a planted core)")
add_custom_target(core_global ALL
  COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CORE_TGT}/target-global
          cargo build --release --features global-widths,${AK_CORE_FEATURES} --manifest-path ${CORE_DIR}/Cargo.toml
  BYPRODUCTS ${CORE_GLOBAL_OUT}/libak_core.so
  COMMENT "cargo build the shared ak-core --features global-widths (a planted core)")
add_custom_target(core_both ALL
  COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CORE_TGT}/target-both
          cargo build --release --features pad-widths,global-widths,${AK_CORE_FEATURES}
          --manifest-path ${CORE_DIR}/Cargo.toml
  BYPRODUCTS ${CORE_BOTH_OUT}/libak_core.so
  COMMENT "cargo build the shared ak-core --features pad-widths,global-widths (a planted core)")

# ---- one arm ------------------------------------------------------------------------
# ak_arm(<name> <exe-source> <std> <impl> <guard> <linkage> <policy> <core-dir>)
function(ak_arm name src std impl guard linkage policy coredir)
  add_executable(${name}
    ${src}
    ${CMAKE_CURRENT_SOURCE_DIR}/src/rt.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/values.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/types.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/build.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/pb_build.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/core_native.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/binding.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/types_borrow.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/binding_borrow.cpp)
  add_dependencies(${name} core core_count core_pad core_global core_both)
  target_include_directories(${name} PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    ${CMAKE_CURRENT_SOURCE_DIR}/src
    ${GEN})
  target_link_libraries(${name} PRIVATE shapes_pb Threads::Threads)
  set_property(TARGET ${name} PROPERTY CXX_STANDARD ${std})
  set_property(TARGET ${name} PROPERTY CXX_STANDARD_REQUIRED ON)
  target_compile_definitions(${name} PRIVATE AK_LINKAGE="${linkage}")
  if(impl STREQUAL "floor")
    target_compile_definitions(${name} PRIVATE AK_FLOOR_IMPL=1)
  endif()
  if(guard STREQUAL "off")
    target_compile_definitions(${name} PRIVATE AK_NO_GUARD=1)
  endif()
  if(policy STREQUAL "lossy")
    target_compile_definitions(${name} PRIVATE AK_DEC_LOSSY=1)
  endif()
  if(linkage STREQUAL "static")
    target_link_libraries(${name} PRIVATE ${coredir}/libak_core.a ${CMAKE_DL_LIBS} m)
  else()
    target_link_libraries(${name} PRIVATE ${coredir}/libak_core.so)
    set_property(TARGET ${name} APPEND_STRING PROPERTY LINK_FLAGS " -Wl,-rpath,${coredir}")
  endif()
  if(linkage STREQUAL "static-lto")
    # R5's hazard, deliberately reproduced: a statically linked core plus -flto is where
    # the entry points can be inlined away with the counting code inside them.
    target_compile_options(${name} PRIVATE -flto)
    set_property(TARGET ${name} APPEND_STRING PROPERTY LINK_FLAGS " -flto")
  endif()
endfunction()

set(CONF ${CMAKE_CURRENT_SOURCE_DIR}/src/conformance.cpp)
set(BENCH ${CMAKE_CURRENT_SOURCE_DIR}/src/bench.cpp)
set(COUNTS ${CMAKE_CURRENT_SOURCE_DIR}/src/counts.cpp)
set(CONC ${CMAKE_CURRENT_SOURCE_DIR}/src/concurrency.cpp)
set(U8 ${CMAKE_CURRENT_SOURCE_DIR}/src/utf8check.cpp)
set(CS ${CMAKE_CURRENT_SOURCE_DIR}/src/contentsets.cpp)

# C18's control: a fixture with one function that MUST be called and one that MUST be
# fused, so gen/boundary.sh's call-site counter is proved able to report both. No
# protobuf, no core, no arm machinery -- it is a test of the checker, not of the slice.
add_executable(fusion_probe ${CMAKE_CURRENT_SOURCE_DIR}/src/fusion_probe.cpp)
set_property(TARGET fusion_probe PROPERTY CXX_STANDARD 17)

# C24: `ak::Dec::skip` over the deprecated GROUP form. A unit test over `include/ak/rt.h`
# alone -- no protobuf, no core, no facade -- so it can be built at every standard level
# and in both implementations cheaply, which README 5.2 requires of anything in the
# decode path. `gen/groupskip.sh` also builds the two PLANTS and requires each to fail.
#
# ak_group(<name> <std> <impl> <plant>)
function(ak_group name std impl plant)
  add_executable(${name} ${CMAKE_CURRENT_SOURCE_DIR}/src/groupskip.cpp)
  target_include_directories(${name} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include)
  set_property(TARGET ${name} PROPERTY CXX_STANDARD ${std})
  set_property(TARGET ${name} PROPERTY CXX_STANDARD_REQUIRED ON)
  target_compile_definitions(${name} PRIVATE AK_GROUP_PLANT=${plant})
  if(impl STREQUAL "floor")
    target_compile_definitions(${name} PRIVATE AK_FLOOR_IMPL=1)
  endif()
endfunction()
ak_group(groupskip_a17 17 target 0)
ak_group(groupskip_b17 17 floor  0)
ak_group(groupskip_c14 14 floor  0)
ak_group(groupskip_c11 11 floor  0)
# The plants. Correctness fixtures, never timed; gen/groupskip.sh inverts their exit code.
ak_group(groupskip_plant_depth 17 target 1)
ak_group(groupskip_plant_i32   17 target 2)

# Correctness gates. README 5.2's arms: a = target impl at the target level; b = FLOOR
# impl at the target level; c = floor impl at the floor level.
ak_arm(conformance_a17_shared ${CONF} 17 target on shared reject ${CORE_OUT})
ak_arm(conformance_c11_shared ${CONF} 11 floor  on shared reject ${CORE_OUT})
ak_arm(conformance_c14_shared ${CONF} 14 floor  on shared reject ${CORE_OUT})
ak_arm(conformance_b17_shared ${CONF} 17 floor  on shared reject ${CORE_OUT})
ak_arm(conformance_a17_static ${CONF} 17 target on static reject ${CORE_OUT})
# R-G7's planted control on the shapes core: the binding skips ak_init (rendered from
# plan.lifecycle). The core is built with init-guard, so every ffi check must FAIL.
ak_arm(conformance_a17_noinit ${CONF} 17 target on shared reject ${CORE_OUT})
target_compile_definitions(conformance_a17_noinit PRIVATE AK_BINDING_SKIP_INIT=1)
# `conformance_a17_lossy` and `bench_a17_lossy` are RETIRED (FIX-PLAN WP5 step 2): the
# decode UTF-8 policy is a generator option (plan.Options.utf8), rendered by the shared
# backends into the native codec AND into the core, not a host build switch. A lossy build
# needs a lossy-lowered plan and a lossy core; the C++ native backend raises on it today.

# Timing. Only arm a produces ratios for the report; b and c are the floor's price.
ak_arm(bench_a17_shared   ${BENCH} 17 target on  shared reject ${CORE_OUT})
ak_arm(bench_a17_static   ${BENCH} 17 target on  static reject ${CORE_OUT})
ak_arm(bench_b17_shared   ${BENCH} 17 floor  on  shared reject ${CORE_OUT})
ak_arm(bench_c11_shared   ${BENCH} 11 floor  on  shared reject ${CORE_OUT})
ak_arm(bench_c14_shared   ${BENCH} 14 floor  on  shared reject ${CORE_OUT})
ak_arm(bench_a17_noguard  ${BENCH} 17 target off shared reject ${CORE_OUT})
# C16: the P1.2 decode outlier, which appeared in rounds 1 and 2 of the `ffi` arm in every
# log this slice took and vanished when the validator changed. Same source, one define, so
# the question "is the outlier the validator?" is answered by a build rather than argued.
ak_arm(bench_a17_scalarv ${BENCH} 17 target on  shared reject ${CORE_OUT})
target_compile_definitions(bench_a17_scalarv PRIVATE AK_UTF8=0)

# R4's across-build control: the same source with a semantically neutral layout
# perturbation, so a drift between two binaries can be measured instead of assumed absent.
ak_arm(bench_a17_perturb  ${BENCH} 17 target on  shared reject ${CORE_OUT})
target_compile_definitions(bench_a17_perturb PRIVATE AK_PERTURB=1)
# ABI v1 open decision 1: the batching predicate priced at a DEARER crossing, so the
# verdict transfers to .NET, FFM and JNI instead of being a fact about 1.85 ns.
ak_arm(bench_a17_tax      ${BENCH} 17 target on  shared reject ${CORE_OUT})
target_compile_definitions(bench_a17_tax PRIVATE AK_CROSSING_TAX=1)

# ABI v1 obligation 12.5: the concurrency suite, plus the three PLANTED builds that show
# it can fail. The plants are the two designs section 6 refused (see include/ak/rt.h); they
# are correctness fixtures, never timed, and `gen/concurrency.sh` inverts their exit code.
ak_arm(conc_a17_shared ${CONC} 17 target on shared reject ${CORE_OUT})
ak_arm(conc_c11_shared ${CONC} 11 floor  on shared reject ${CORE_OUT})
ak_arm(conc_a17_static ${CONC} 17 target on static reject ${CORE_OUT})
# Each planted build plants BOTH encoders the same way: `ak::Enc` by define, and the core
# by linking its planted feature build (R-D7). AK_CONC_CORE_PLANT tells the binary which
# core it linked, so its first lines say so and main() can require the ffi arm to fail.
ak_arm(conc_a17_pad    ${CONC} 17 target on shared reject ${CORE_PAD_OUT})
target_compile_definitions(conc_a17_pad PRIVATE AK_CONC_PAD=1 AK_CONC_CORE_PLANT="pad-widths")
ak_arm(conc_a17_global ${CONC} 17 target on shared reject ${CORE_GLOBAL_OUT})
target_compile_definitions(conc_a17_global PRIVATE AK_CONC_GLOBAL=1 AK_CONC_CORE_PLANT="global-widths")
ak_arm(conc_a17_both   ${CONC} 17 target on shared reject ${CORE_BOTH_OUT})
target_compile_definitions(conc_a17_both PRIVATE AK_CONC_PAD=1 AK_CONC_GLOBAL=1
                           AK_CONC_CORE_PLANT="pad-widths,global-widths")
# The core planted ALONE, `ak::Enc` shipped: the ffi arm has to fail on its own, with the
# native arm at 0, or the suite is only ever seeing the native encoder.
ak_arm(conc_a17_corepad ${CONC} 17 target on shared reject ${CORE_PAD_OUT})
target_compile_definitions(conc_a17_corepad PRIVATE AK_CONC_CORE_PLANT="pad-widths")
# And the gate refusal: a bench whose `ffi-valtc` transcoder is replaced by one that
# refuses every string (R-D5). The bench must refuse to time that arm and exit non-zero.
# A correctness fixture, never a timing.
ak_arm(bench_a17_gateplant ${BENCH} 17 target on shared reject ${CORE_OUT})
target_compile_definitions(bench_a17_gateplant PRIVATE AK_GATE_PLANT=1)

# ABI v1 open decision 3's validator, which is what the decode-side number was really
# about. Correctness first: an exhaustive differential test against an independent oracle,
# then the timing, both in one process. Also built at the floor, because the DFA is the one
# a core would ship and a core ships to C++11 hosts too.
ak_arm(contentsets_a17 ${CS} 17 target on shared reject ${CORE_OUT})

# W8 / FIX-PLAN WP5 item 6.1: the FULL conformance corpus, through the C ABI (the core
# generated for the corpus's READER schema, `--features corpus,init-guard`, its own ABI and
# its own header in corpus/include) and core-native, unknown fields dropped and retained.
# Everything but corpus_main.cpp / corpus_harness.h is rendered from the corpus plan by the
# shared C++ backend (gen/generate.py writes corpus/**). A different ABI from the shapes
# core, so it is never linked with a shapes facade. gen/corpus_all.py takes the verdict,
# one child process per row under a timeout. The previous subset harness (src/corpus.cpp,
# gen/corpus.py: 213 in-scope rows of the shapes roots, native + ffi + pb) is retired.
set(CORE_CORPUS_OUT "${CORE_TGT}/target-corpus/release")
add_custom_target(core_corpus ALL
  COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CORE_TGT}/target-corpus
          cargo build --release --features corpus,${AK_CORE_FEATURES}
          --manifest-path ${CORE_DIR}/Cargo.toml
  BYPRODUCTS ${CORE_CORPUS_OUT}/libak_core.so ${CORE_CORPUS_OUT}/libak_core.a
  COMMENT "cargo build the shared ak-core --features corpus,init-guard (corpus reader ABI)")
set(CORPUS_DIR ${CMAKE_CURRENT_SOURCE_DIR}/corpus)
# ak_corpus(<name> <std> <impl> <linkage>)
function(ak_corpus name std impl linkage)
  add_executable(${name}
    ${CORPUS_DIR}/src/corpus_main.cpp
    ${CORPUS_DIR}/src/generated/dispatch.cpp
    ${CORPUS_DIR}/src/generated/types.cpp
    ${CORPUS_DIR}/src/generated/core_native.cpp
    ${CORPUS_DIR}/src/generated/core_native_retain.cpp
    ${CORPUS_DIR}/src/generated/binding.cpp
    ${CORPUS_DIR}/src/generated/project.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/rt.cpp)
  add_dependencies(${name} core_corpus)
  # ORDER MATTERS: corpus/include first, so "ak_abi.h" and "generated/ak_layout.h" are the
  # corpus ABI's and never the shapes one in include/.
  target_include_directories(${name} PRIVATE ${CORPUS_DIR}/include ${CORPUS_DIR}/src
    ${CMAKE_CURRENT_SOURCE_DIR}/include ${Protobuf_INCLUDE_DIRS})
  target_link_libraries(${name} PRIVATE ${Protobuf_LIBRARIES} Threads::Threads)
  set_property(TARGET ${name} PROPERTY CXX_STANDARD ${std})
  set_property(TARGET ${name} PROPERTY CXX_STANDARD_REQUIRED ON)
  if(impl STREQUAL "floor")
    target_compile_definitions(${name} PRIVATE AK_FLOOR_IMPL=1)
  endif()
  if(linkage STREQUAL "static")
    target_link_libraries(${name} PRIVATE ${CORE_CORPUS_OUT}/libak_core.a ${CMAKE_DL_LIBS} m)
  else()
    target_link_libraries(${name} PRIVATE ${CORE_CORPUS_OUT}/libak_core.so)
    set_property(TARGET ${name} APPEND_STRING PROPERTY LINK_FLAGS " -Wl,-rpath,${CORE_CORPUS_OUT}")
  endif()
endfunction()
ak_corpus(corpus_all_a17 17 target shared)
ak_corpus(corpus_all_c11 11 floor  shared)
ak_corpus(corpus_all_c14 14 floor  shared)
ak_corpus(corpus_all_a17_static 17 target static)
# The planted control: the binding skips ak_init. Against the init-guard core every ffi
# arm must fail (gen/corpus_all.py --expect-fail noinit).
ak_corpus(corpus_all_noinit 17 target shared)
target_compile_definitions(corpus_all_noinit PRIVATE AK_BINDING_SKIP_INIT=1)

ak_arm(utf8check_a17 ${U8} 17 target on shared reject ${CORE_OUT})
ak_arm(utf8check_c11 ${U8} 11 floor  on shared reject ${CORE_OUT})

# R5: the crossing counts come from a counting CORE, never from the timed binary.
ak_arm(counts_a17_shared ${COUNTS} 17 target on shared reject ${CORE_COUNT_OUT})
ak_arm(counts_a17_static ${COUNTS} 17 target on static reject ${CORE_COUNT_OUT})
# AK_COUNTING lets the HOST side report a crossing the core cannot see (the host
# transcoder). It is defined only on the counting binaries, never on a timed one.
target_compile_definitions(counts_a17_shared PRIVATE AK_COUNTING=1)
target_compile_definitions(counts_a17_static PRIVATE AK_COUNTING=1)

# R5's hazard, built on purpose so the check that catches it can be seen working.
# Two halves, because in C++ they are different questions:
#  - over a STATICALLY LINKED CORE: can -flto inline an ABI entry point away, counters
#    and all? (`counts_a17_static_lto`)
#  - over the NO-BOUNDARY CONTROL: can -flto fuse the traversal into the benchmark loop,
#    which would make every subtraction against it invalid? (`bench_a17_shared_lto`)
ak_arm(counts_a17_static_lto ${COUNTS} 17 target on static-lto reject ${CORE_COUNT_OUT})
target_compile_definitions(counts_a17_static_lto PRIVATE AK_COUNTING=1)
ak_arm(bench_a17_shared_lto ${BENCH} 17 target on shared reject ${CORE_OUT})
target_compile_options(bench_a17_shared_lto PRIVATE -flto)
set_property(TARGET bench_a17_shared_lto APPEND_STRING PROPERTY LINK_FLAGS " -flto")

# ---- the RPC arm (ABI v1 section 9, design/SHAPES.md's RPC arm) ---------------------
# Optional: it needs libgrpc++-dev and a core built with the `rpc` feature, and neither is
# wanted by the codec arms. -DAK_RPC=ON turns it on.
if(AK_RPC)
  find_package(PkgConfig REQUIRED)
  pkg_check_modules(GRPCPP REQUIRED grpc++)
  find_program(GRPC_CPP_PLUGIN grpc_cpp_plugin REQUIRED)
  set(CORE_RPC_OUT "${CORE_TGT}/target-rpc/release")
  add_custom_target(core_rpc ALL
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CORE_TGT}/target-rpc
            cargo build --release --features rpc --manifest-path ${CORE_DIR}/Cargo.toml
    BYPRODUCTS ${CORE_RPC_OUT}/libak_core.so
    COMMENT "cargo build the shared ak-core --features rpc")
  add_custom_command(
    OUTPUT ${GEN}/shapes_svc.pb.cc ${GEN}/shapes_svc.pb.h
           ${GEN}/shapes_svc.grpc.pb.cc ${GEN}/shapes_svc.grpc.pb.h
    COMMAND ${Protobuf_PROTOC_EXECUTABLE} --cpp_out=${GEN}
            -I${SCHEMA} -I${CMAKE_CURRENT_SOURCE_DIR}/proto
            ${CMAKE_CURRENT_SOURCE_DIR}/proto/shapes_svc.proto
    COMMAND ${Protobuf_PROTOC_EXECUTABLE} --grpc_out=${GEN}
            --plugin=protoc-gen-grpc=${GRPC_CPP_PLUGIN}
            -I${SCHEMA} -I${CMAKE_CURRENT_SOURCE_DIR}/proto
            ${CMAKE_CURRENT_SOURCE_DIR}/proto/shapes_svc.proto
    DEPENDS ${CMAKE_CURRENT_SOURCE_DIR}/proto/shapes_svc.proto ${SCHEMA}/shapes.proto
    COMMENT "protoc + grpc_cpp_plugin shapes_svc.proto")
  set(CORE_COUNT_RPC_OUT "${CORE_TGT}/target-rpc-count/release")
  add_custom_target(core_rpc_count ALL
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CORE_TGT}/target-rpc-count
            cargo build --release --features rpc,count --manifest-path ${CORE_DIR}/Cargo.toml
    BYPRODUCTS ${CORE_COUNT_RPC_OUT}/libak_core.so
    COMMENT "cargo build the shared ak-core --features rpc,count")

  # Three binaries over one source set, and never one binary with three jobs:
  #   rpcbench   the timed grid                     -- the core WITHOUT --features count
  #   rpccounts  R5's crossing counts               -- the core WITH --features count
  #   rpcflow    the flow-control probe, no timing  -- either; it links the non-counting one
  # A counting core inside a timed binary is what R5 forbids.
  set(RPC_COMMON
    ${CMAKE_CURRENT_SOURCE_DIR}/src/rpc_common.cpp
    ${GEN}/shapes_svc.pb.cc ${GEN}/shapes_svc.grpc.pb.cc
    ${CMAKE_CURRENT_SOURCE_DIR}/src/rt.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/values.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/types.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/build.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/pb_build.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/binding.cpp)

  # ak_rpc_bin(<name> <source> <core-dir> <core-target>)
  function(ak_rpc_bin name src coredir coretgt)
    add_executable(${name} ${src} ${RPC_COMMON})
    add_dependencies(${name} ${coretgt})
    target_include_directories(${name} PRIVATE
      ${CMAKE_CURRENT_SOURCE_DIR}/include ${CMAKE_CURRENT_SOURCE_DIR}/src ${GEN}
      ${GRPCPP_INCLUDE_DIRS})
    target_link_libraries(${name} PRIVATE shapes_pb Threads::Threads
      ${GRPCPP_LIBRARIES} ${coredir}/libak_core.so)
    target_link_directories(${name} PRIVATE ${GRPCPP_LIBRARY_DIRS})
    set_property(TARGET ${name} PROPERTY CXX_STANDARD 17)
    target_compile_definitions(${name} PRIVATE AK_LINKAGE="shared")
    set_property(TARGET ${name} APPEND_STRING PROPERTY LINK_FLAGS " -Wl,-rpath,${coredir}")
  endfunction()

  ak_rpc_bin(rpcbench  ${CMAKE_CURRENT_SOURCE_DIR}/src/rpcbench.cpp  ${CORE_RPC_OUT}       core_rpc)
  ak_rpc_bin(rpcflow   ${CMAKE_CURRENT_SOURCE_DIR}/src/rpcflow.cpp   ${CORE_RPC_OUT}       core_rpc)
  ak_rpc_bin(rpccounts ${CMAKE_CURRENT_SOURCE_DIR}/src/rpccounts.cpp ${CORE_COUNT_RPC_OUT} core_rpc_count)

  # ---- design/CAMPAIGN.md (W11): the campaign's binaries -----------------------------
  # gen/run_campaign.sh drives them; nothing here prints a summary. The RPC client and the
  # server are TWO processes, pinned by the runner to AK_CPU_CLIENT and AK_CPU_SERVER.
  # The rpc core is built with init-guard like every other core (requirement 6).
  set(CORE_CAMP_OUT "${CORE_TGT}/target-camp/release")
  add_custom_target(core_camp ALL
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CORE_TGT}/target-camp
            cargo build --release --features rpc,${AK_CORE_FEATURES} --manifest-path ${CORE_DIR}/Cargo.toml
    BYPRODUCTS ${CORE_CAMP_OUT}/libak_core.so
    COMMENT "cargo build the shared ak-core --features rpc,init-guard (campaign)")
  ak_rpc_bin(campaign_rpc    ${CMAKE_CURRENT_SOURCE_DIR}/src/campaign_rpc.cpp    ${CORE_CAMP_OUT} core_camp)
  ak_rpc_bin(campaign_server ${CMAKE_CURRENT_SOURCE_DIR}/src/campaign_server.cpp ${CORE_CAMP_OUT} core_camp)
  ak_rpc_bin(campaign_calib  ${CMAKE_CURRENT_SOURCE_DIR}/src/campaign_calib.cpp  ${CORE_CAMP_OUT} core_camp)
  # The codec suite links the same core as the codec gates (CORE_OUT: init-guard, no rpc)
  # and grpc++ for the production path's SerializationTraits.
  ak_arm(campaign_codec ${CMAKE_CURRENT_SOURCE_DIR}/src/campaign_codec.cpp 17 target on shared reject ${CORE_OUT})
  target_sources(campaign_codec PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/core_native_retain.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/touch.cpp)
  target_include_directories(campaign_codec PRIVATE ${GRPCPP_INCLUDE_DIRS})
  target_link_directories(campaign_codec PRIVATE ${GRPCPP_LIBRARY_DIRS})
  target_link_libraries(campaign_codec PRIVATE ${GRPCPP_LIBRARIES})
  # CAMPAIGN.md requirement 22a: the codec suite's timing is Google Benchmark (apt
  # libbenchmark-dev; the version is printed in every log).
  find_package(benchmark REQUIRED)
  target_link_libraries(campaign_codec PRIVATE benchmark::benchmark)
  target_compile_definitions(campaign_codec PRIVATE AK_GBENCH_VERSION="${benchmark_VERSION}")
endif()

# ---- the upb arm: a CEILING, not a candidate ----------------------------------------
# -DAK_UPB=ON after running gen/fetch_upb.sh. protobuf C++ stays the incumbent every ratio
# in this slice is against; upb bounds how fast a C protobuf can be, which bounds the
# core's wins from the direction R2's memcpy floor does not reach.
if(AK_UPB)
  set(UPB_SRC "${CMAKE_CURRENT_SOURCE_DIR}/build/upb-src")
  set(UPB_BUILD "${CMAKE_CURRENT_SOURCE_DIR}/build/upb-build")
  set(UPB_LIBS "${UPB_BUILD}/libupb.a")
  if(AK_UPB_VARIANT)
    set(UPB_BUILD "${CMAKE_CURRENT_SOURCE_DIR}/build/upb-build-${AK_UPB_VARIANT}")
    set(UPB_LIBS "${UPB_BUILD}/libupb.a")
  endif()
  if(NOT AK_UPB_TAG)
    set(AK_UPB_TAG "v25.3")
  endif()
  add_custom_command(
    OUTPUT ${GEN}/shapes.desc
    COMMAND ${Protobuf_PROTOC_EXECUTABLE} --descriptor_set_out=${GEN}/shapes.desc
            -I${SCHEMA} ${SCHEMA}/shapes.proto
    DEPENDS ${SCHEMA}/shapes.proto
    COMMENT "protoc --descriptor_set_out shapes.desc (upb reflection input)")
  add_custom_target(shapes_desc DEPENDS ${GEN}/shapes.desc)
  add_executable(upbbench
    ${CMAKE_CURRENT_SOURCE_DIR}/src/upbbench.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/rt.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/values.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/types.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/build.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/pb_build.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/core_native.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/binding.cpp)
  add_dependencies(upbbench core shapes_desc)
  target_include_directories(upbbench PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/include ${CMAKE_CURRENT_SOURCE_DIR}/src ${GEN}
    ${UPB_SRC}/upb/reflection/stage0 ${UPB_SRC}
    ${CMAKE_CURRENT_SOURCE_DIR}/build/upbinc)
  target_link_libraries(upbbench PRIVATE shapes_pb Threads::Threads
    ${CORE_OUT}/libak_core.so ${UPB_LIBS})
  set_property(TARGET upbbench PROPERTY CXX_STANDARD 17)
  if(NOT AK_UPB_CC)
    set(AK_UPB_CC "cc (gcc 13.3.0)")
  endif()
  if(NOT AK_UPB_FT)
    set(AK_UPB_FT 0)
  endif()
  target_compile_definitions(upbbench PRIVATE AK_LINKAGE="shared"
    AK_UPB_VERSION="${AK_UPB_TAG}" AK_UPB_CC="${AK_UPB_CC}"
    AK_UPB_FASTTABLE_BUILD=${AK_UPB_FT})
  set_property(TARGET upbbench APPEND_STRING PROPERTY LINK_FLAGS " -Wl,-rpath,${CORE_OUT}")
endif()

# README 5.1's hard stop, checked mechanically: two TUs at two standard levels, linked.
add_executable(odrcheck
  ${CMAKE_CURRENT_SOURCE_DIR}/src/odr_main.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/src/odr_a.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/src/odr_b.cpp
  ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/types.cpp)
target_include_directories(odrcheck PRIVATE
  ${CMAKE_CURRENT_SOURCE_DIR}/include ${CMAKE_CURRENT_SOURCE_DIR}/src ${GEN})
set_source_files_properties(${CMAKE_CURRENT_SOURCE_DIR}/src/odr_a.cpp
  PROPERTIES COMPILE_OPTIONS "-std=c++11")
set_source_files_properties(${CMAKE_CURRENT_SOURCE_DIR}/src/odr_b.cpp
  PROPERTIES COMPILE_OPTIONS "-std=c++17")
set_property(TARGET odrcheck PROPERTY CXX_STANDARD 17)
if(AK_ODR_BREAK)
  target_compile_definitions(odrcheck PRIVATE AK_ODR_BREAK=1)
endif()
