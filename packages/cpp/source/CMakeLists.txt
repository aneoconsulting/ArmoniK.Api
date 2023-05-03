cmake_minimum_required(VERSION 3.18)

set(version 0.1.0)

set(SOLUTION_NAME Armonik.Api)
if (UNIX)

    set(BUILD_DIR "/app/build")
    set(PROTO_FILES_DIR "/app/proto")

elseif (WIN32)

    set(BUILD_DIR "${CMAKE_SOURCE_DIR}/../build")
    set(PROTO_FILES_DIR "${CMAKE_SOURCE_DIR}/../../../Protos/V1")
    list(APPEND CMAKE_PREFIX_PATH "${CMAKE_SOURCE_DIR}/../tools/win64/lib/cmake/grpc")

endif (UNIX)

project(${SOLUTION_NAME} C CXX)


option(BUILD_CLIENT OFF)


# make cache variables for install destinations
include(GNUInstallDirs)
include(CMakePackageConfigHelpers)


set(CMAKE_CXX_STANDARD 17)




add_subdirectory(ArmoniK.Api.Common)
add_subdirectory(ArmoniK.Api.Worker)
add_subdirectory(ArmoniK.Api.Worker.Tests)

if (BUILD_CLIENT OR WIN32)
    add_subdirectory(ArmoniK.Api.Client)
    add_subdirectory(ArmoniK.Api.Tests) 
endif()


