cmake_minimum_required(VERSION 3.22)

set(version 0.1.0)

set(SOLUTION_NAME ArmoniK.Api)
project(${SOLUTION_NAME} C CXX)
if (UNIX)
    set(BUILD_DIR "/app/build")
    set(PROTO_FILES_DIR "/app/proto")
elseif (WIN32)
    set(BUILD_DIR "${CMAKE_SOURCE_DIR}/build")
    set(PROTO_FILES_DIR "${CMAKE_SOURCE_DIR}/../../Protos/V1")
    list(APPEND CMAKE_PREFIX_PATH "${CMAKE_SOURCE_DIR}/tools/win64/lib/cmake/grpc")
    list(APPEND CMAKE_PREFIX_PATH "${CMAKE_SOURCE_DIR}/tools/win64/cmake")
    set(Protobuf_USE_STATIC_LIBS ON)
endif (UNIX)

option(BUILD_TEST "Build tests" OFF)
option(BUILD_CLIENT "Build client" ON)
option(BUILD_WORKER "Build worker" ON)

# make cache variables for install destinations
include(GNUInstallDirs)
include(CMakePackageConfigHelpers)

set(CMAKE_CXX_STANDARD 17)

function(setup_options project_name_param)

file(READ /etc/issue ${PROJECT_NAME}_ETCISSUE_CONTENT)
string(FIND "${${project_name_param}_ETCISSUE_CONTENT}" "Alpine" IS_ALPINE)

if(MSVC)
    target_compile_options(${project_name_param} PRIVATE /W4)
else()
    if(CMAKE_BUILD_TYPE MATCHES DEBUG AND IS_ALPINE EQUAL -1)
        target_compile_options(${project_name_param} PRIVATE -Wall -Wextra -Wpedantic -fsanitize=undefined,address)
    else ()
        target_compile_options(${project_name_param} PRIVATE -Wall -Wextra -Wpedantic)
    endif()
endif()

if(IS_ALPINE EQUAL -1)
    message(STATUS "Not found Alpine distribution " ${IS_ALPINE})
else()
    message(STATUS "Found Alpine distribution " ${IS_ALPINE})
endif()

if(CMAKE_BUILD_TYPE MATCHES DEBUG AND IS_ALPINE EQUAL -1)
    target_link_options(${project_name_param} PRIVATE -fsanitize=undefined,address)
endif()
endfunction()

add_subdirectory(ArmoniK.Api.Common)
if(BUILD_WORKER)
    add_subdirectory(ArmoniK.Api.Worker)
endif()

if(BUILD_CLIENT)
    add_subdirectory(ArmoniK.Api.Client)
endif()


if (BUILD_TEST)
    if(BUILD_WORKER)
        add_subdirectory(ArmoniK.Api.Worker.Tests)
    endif()
    if(BUILD_CLIENT)
        add_subdirectory(ArmoniK.Api.Tests)
    endif()
endif()

