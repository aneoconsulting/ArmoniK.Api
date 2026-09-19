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
set(CMAKE_CXX_FLAGS_RELEASE "-O2 -g")

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
# Built by cargo, from the codec emitted by the RUST slice's own emitter (R1). Two builds:
# the timed one and the counting one (README R5), which never share a binary.
set(CORE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/core")
set(CORE_OUT "${CORE_DIR}/target/release")
set(CORE_COUNT_OUT "${CORE_DIR}/target-count/release")

add_custom_target(core ALL
  COMMAND cargo build --release --manifest-path ${CORE_DIR}/Cargo.toml
  BYPRODUCTS ${CORE_OUT}/libak_core_cpp.so ${CORE_OUT}/libak_core_cpp.a
  COMMENT "cargo build ak-core-cpp (cdylib + staticlib)")

add_custom_target(core_count ALL
  COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CORE_DIR}/target-count
          cargo build --release --features count --manifest-path ${CORE_DIR}/Cargo.toml
  BYPRODUCTS ${CORE_COUNT_OUT}/libak_core_cpp.so
  COMMENT "cargo build ak-core-cpp --features count")

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
    ${CMAKE_CURRENT_SOURCE_DIR}/src/generated/binding.cpp)
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
    target_link_libraries(${name} PRIVATE ${coredir}/libak_core_cpp.a ${CMAKE_DL_LIBS} m)
  else()
    target_link_libraries(${name} PRIVATE ${coredir}/libak_core_cpp.so)
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

# R5: the crossing counts come from a counting CORE, never from the timed binary.
ak_arm(counts_a17_shared ${COUNTS} 17 target on shared reject ${CORE_COUNT_OUT})
ak_arm(counts_a17_static ${COUNTS} 17 target on static reject ${CORE_COUNT_OUT})

# R5's hazard, built on purpose so the check that catches it can be seen working.
# Two halves, because in C++ they are different questions:
#  - over a STATICALLY LINKED CORE: can -flto inline an ABI entry point away, counters
#    and all? (`counts_a17_static_lto`)
#  - over the NO-BOUNDARY CONTROL: can -flto fuse the traversal into the benchmark loop,
#    which would make every subtraction against it invalid? (`bench_a17_shared_lto`)
ak_arm(counts_a17_static_lto ${COUNTS} 17 target on static-lto reject ${CORE_COUNT_OUT})
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
  set(CORE_RPC_OUT "${CORE_DIR}/target-rpc/release")
  add_custom_target(core_rpc ALL
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CORE_DIR}/target-rpc
            cargo build --release --features rpc --manifest-path ${CORE_DIR}/Cargo.toml
    BYPRODUCTS ${CORE_RPC_OUT}/libak_core_cpp.so
    COMMENT "cargo build ak-core-cpp --features rpc")
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
    ${GRPCPP_LIBRARIES} ${CORE_RPC_OUT}/libak_core_cpp.so)
  target_link_directories(rpcbench PRIVATE ${GRPCPP_LIBRARY_DIRS})
  set_property(TARGET rpcbench PROPERTY CXX_STANDARD 17)
  target_compile_definitions(rpcbench PRIVATE AK_LINKAGE="shared")
  set_property(TARGET rpcbench APPEND_STRING PROPERTY LINK_FLAGS " -Wl,-rpath,${CORE_RPC_OUT}")
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
