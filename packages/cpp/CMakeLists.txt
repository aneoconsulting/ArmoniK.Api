cmake_minimum_required(VERSION 3.22)
if (POLICY CMP0135)
    cmake_policy(SET CMP0135 OLD)
endif ()
set(version 4.0.0)
string(REGEX MATCHALL "[0-9]+" version_list ${version})
list(GET version_list 0 version_major)
list(GET version_list 1 version_minor)
list(GET version_list 2 version_patch)
message(STATUS "ArmoniK Version ${version}")

set(SOLUTION_NAME ArmoniK.Api)
project(${SOLUTION_NAME} C CXX)
if(NOT DEFINED BUILD_DIR)
    set(BUILD_DIR "${CMAKE_CURRENT_BINARY_DIR}")
endif()
if(NOT DEFINED PROTO_FILES_DIR)
    set(PROTO_FILES_DIR "${CMAKE_SOURCE_DIR}/../../Protos/V1")
endif()
if (WIN32)
    list(APPEND CMAKE_PREFIX_PATH "${CMAKE_SOURCE_DIR}/tools/win64/lib/cmake/grpc")
    list(APPEND CMAKE_PREFIX_PATH "${CMAKE_SOURCE_DIR}/tools/win64/cmake")
    set(Protobuf_USE_STATIC_LIBS ON)
endif ()

option(BUILD_TEST "Build tests" OFF)
option(BUILD_CLIENT "Build client" ON)
option(BUILD_WORKER "Build worker" ON)
option(BUILD_SHARED_LIBS "Build using shared libraries" OFF)

# make cache variables for install destinations
include(GNUInstallDirs)
include(CMakePackageConfigHelpers)
include(Dependencies.cmake)

function(setup_options project_name_param)


if(MSVC)
    target_compile_options(${project_name_param} PRIVATE /W4)
else()
    
    file(READ /etc/issue ${PROJECT_NAME}_ETCISSUE_CONTENT)
    string(FIND "${${project_name_param}_ETCISSUE_CONTENT}" "Alpine" IS_ALPINE)

    if(CMAKE_BUILD_TYPE MATCHES DEBUG AND IS_ALPINE EQUAL -1)
        target_compile_options(${project_name_param} PRIVATE -Wall -Wextra -Wpedantic -fsanitize=undefined,address)
    else ()
        target_compile_options(${project_name_param} PRIVATE -Wall -Wextra -Wpedantic)
    endif()
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

include(Packaging.cmake)
include(CPack)
