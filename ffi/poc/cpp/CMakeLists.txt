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
set(CORE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../codec/crates/ak-core")
set(CORE_TGT "${CMAKE_CURRENT_SOURCE_DIR}/core-build")
set(CORE_OUT "${CORE_TGT}/target/release")
set(CORE_COUNT_OUT "${CORE_TGT}/target-count/release")

add_custom_target(core ALL
  COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CORE_TGT}/target
          cargo build --release --manifest-path ${CORE_DIR}/Cargo.toml
  BYPRODUCTS ${CORE_OUT}/libak_core.so ${CORE_OUT}/libak_core.a
  COMMENT "cargo build the shared ak-core (cdylib + staticlib)")

add_custom_target(core_count ALL
  COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CORE_TGT}/target-count
          cargo build --release --features count --manifest-path ${CORE_DIR}/Cargo.toml
  BYPRODUCTS ${CORE_COUNT_OUT}/libak_core.so
  COMMENT "cargo build the shared ak-core --features count")

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
  add_dependencies(${name} core core_count)
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
ak_arm(conformance_a17_lossy  ${CONF} 17 target on shared lossy  ${CORE_OUT})

# Timing. Only arm a produces ratios for the report; b and c are the floor's price.
ak_arm(bench_a17_shared   ${BENCH} 17 target on  shared reject ${CORE_OUT})
ak_arm(bench_a17_static   ${BENCH} 17 target on  static reject ${CORE_OUT})
ak_arm(bench_b17_shared   ${BENCH} 17 floor  on  shared reject ${CORE_OUT})
ak_arm(bench_c11_shared   ${BENCH} 11 floor  on  shared reject ${CORE_OUT})
ak_arm(bench_c14_shared   ${BENCH} 14 floor  on  shared reject ${CORE_OUT})
ak_arm(bench_a17_noguard  ${BENCH} 17 target off shared reject ${CORE_OUT})
ak_arm(bench_a17_lossy    ${BENCH} 17 target on  shared lossy  ${CORE_OUT})
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
ak_arm(conc_a17_pad    ${CONC} 17 target on shared reject ${CORE_OUT})
target_compile_definitions(conc_a17_pad PRIVATE AK_CONC_PAD=1)
ak_arm(conc_a17_global ${CONC} 17 target on shared reject ${CORE_OUT})
target_compile_definitions(conc_a17_global PRIVATE AK_CONC_GLOBAL=1)
ak_arm(conc_a17_both   ${CONC} 17 target on shared reject ${CORE_OUT})
target_compile_definitions(conc_a17_both PRIVATE AK_CONC_PAD=1 AK_CONC_GLOBAL=1)

# ABI v1 open decision 3's validator, which is what the decode-side number was really
# about. Correctness first: an exhaustive differential test against an independent oracle,
# then the timing, both in one process. Also built at the floor, because the DFA is the one
# a core would ship and a core ships to C++11 hosts too.
ak_arm(contentsets_a17 ${CS} 17 target on shared reject ${CORE_OUT})

# W8: this slice as a consumer of ffi/corpus. The oracle byte identity against a
# schema-generated manifest cannot be, because the manifest and the codec come from the
# same description. Built at the target level and at the C++11 FLOOR, because the floor is
# a correctness gate and this is correctness.
# The projector is added to THESE TWO TARGETS ONLY, never to `ak_arm`'s common source
# list: linking a file nothing calls into every timed binary perturbs its layout, and R4's
# own drift control says a neutral layout perturbation is worth up to 0.240 of a ratio.
ak_arm(corpus_a17_shared ${CMAKE_CURRENT_SOURCE_DIR}/src/corpus.cpp 17 target on shared reject ${CORE_OUT})
ak_arm(corpus_c11_shared ${CMAKE_CURRENT_SOURCE_DIR}/src/corpus.cpp 11 floor  on shared reject ${CORE_OUT})
target_sources(corpus_a17_shared PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/project.cpp)
target_sources(corpus_c11_shared PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/project.cpp)

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
  add_executable(rpcbench
    ${CMAKE_CURRENT_SOURCE_DIR}/src/rpcbench.cpp
    ${GEN}/shapes_svc.pb.cc ${GEN}/shapes_svc.grpc.pb.cc
    ${CMAKE_CURRENT_SOURCE_DIR}/src/rt.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/values.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/types.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/build.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/pb_build.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/binding.cpp)
  add_dependencies(rpcbench core_rpc)
  target_include_directories(rpcbench PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/include ${CMAKE_CURRENT_SOURCE_DIR}/src ${GEN}
    ${GRPCPP_INCLUDE_DIRS})
  target_link_libraries(rpcbench PRIVATE shapes_pb Threads::Threads
    ${GRPCPP_LIBRARIES} ${CORE_RPC_OUT}/libak_core.so)
  target_link_directories(rpcbench PRIVATE ${GRPCPP_LIBRARY_DIRS})
  set_property(TARGET rpcbench PROPERTY CXX_STANDARD 17)
  target_compile_definitions(rpcbench PRIVATE AK_LINKAGE="shared")
  set_property(TARGET rpcbench APPEND_STRING PROPERTY LINK_FLAGS " -Wl,-rpath,${CORE_RPC_OUT}")
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
