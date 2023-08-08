# make cache variables for install destinations
include(GNUInstallDirs)
include(CMakePackageConfigHelpers)

set(PROJECT_NAME ArmoniK.Api.Common)
set(NAMESPACE ArmoniK::Api::Common)
set(ARMONIK_INSTALL_INCLUDE_DIR ${CMAKE_INSTALL_INCLUDEDIR}/armonik/common)
set(PROJECT_BUILD_DIR ${BUILD_DIR}/${PROJECT_NAME})

set(PROTO_FILES
    "objects.proto"
    "result_status.proto"
    "task_status.proto"
    "session_status.proto"
    "sort_direction.proto"
    "tasks_fields.proto"
    "tasks_filters.proto"
    "applications_fields.proto"
    "applications_filters.proto"
    "sessions_fields.proto"
    "sessions_filters.proto"
    "results_fields.proto"
    "results_filters.proto"
    "partitions_fields.proto"
    "partitions_filters.proto"
    "filters_common.proto")

list(TRANSFORM PROTO_FILES PREPEND "${PROTO_FILES_DIR}/")

set(CMAKE_FIND_DEBUG_MODE FALSE)
# Trouver les packages requis
find_package(Protobuf REQUIRED)
find_package(gRPC CONFIG REQUIRED)
find_package(Threads)

include(FetchContent)

FETCHCONTENT_DECLARE(
        simdjson
        URL https://github.com/simdjson/simdjson/archive/refs/tags/v3.2.1.tar.gz
)

FetchContent_GetProperties(simdjson)
if(NOT simdjson_POPULATED)
    FetchContent_Populate(simdjson)
endif()

FetchContent_Declare(fmt
  GIT_REPOSITORY https://github.com/fmtlib/fmt.git
  GIT_TAG 10.0.0
)
FetchContent_MakeAvailable(fmt)

set(CMAKE_FIND_DEBUG_MODE FALSE)

SET(SOURCES_FILES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/source")
SET(HEADER_FILES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/header")

FILE(GLOB_RECURSE SRC_CLIENT_FILES ${SOURCES_FILES_DIR}/*.cpp)
FILE(GLOB_RECURSE HEADER_CLIENT_FILES ${HEADER_FILES_DIR}/*.h)

file(MAKE_DIRECTORY ${PROJECT_BUILD_DIR})

add_library(${PROJECT_NAME} STATIC ${PROTO_GENERATED_FILES} ${SRC_CLIENT_FILES} ${HEADER_CLIENT_FILES} ${simdjson_SOURCE_DIR}/singleheader/simdjson.cpp ${simdjson_SOURCE_DIR}/singleheader/simdjson.h)

target_link_libraries(${PROJECT_NAME} PUBLIC protobuf::libprotobuf gRPC::grpc++_unsecure)
target_link_libraries(${PROJECT_NAME} PRIVATE fmt::fmt)
target_compile_definitions(${PROJECT_NAME} PUBLIC API_COMMON_NAMESPACE=${NAMESPACE})

setup_options(${PROJECT_NAME})

set(PROTO_BINARY_DIR "${PROJECT_BUILD_DIR}")
set(PROTO_IMPORT_DIRS "${PROTO_FILES_DIR}")

protobuf_generate(
    TARGET ${PROJECT_NAME}
    PROTOS ${PROTO_FILES}
    OUT_VAR PROTO_GENERATED_FILES
    APPEND_PATH "./"
    IMPORT_DIRS ${PROTO_IMPORT_DIRS}
    PROTOC_OUT_DIR "${PROTO_BINARY_DIR}")
set_source_files_properties(${PROTO_GENERATED_FILES} PROPERTIES SKIP_UNITY_BUILD_INCLUSION on)

get_target_property(grpc_cpp_plugin_location gRPC::grpc_cpp_plugin LOCATION)

target_include_directories(${PROJECT_NAME}
                           PUBLIC
                           "$<BUILD_INTERFACE:${simdjson_SOURCE_DIR}/singleheader/>"
                           "$<BUILD_INTERFACE:${PROJECT_BUILD_DIR}>"
                           "$<BUILD_INTERFACE:${HEADER_FILES_DIR}>"
                           "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>")

set_property(TARGET ${PROJECT_NAME} PROPERTY VERSION ${version})
set_property(TARGET ${PROJECT_NAME} PROPERTY SOVERSION 0)
set_property(TARGET ${PROJECT_NAME} PROPERTY
    INTERFACE_${PROJECT_NAME}_MAJOR_VERSION 0)
set_property(TARGET ${PROJECT_NAME} APPEND PROPERTY
    COMPATIBLE_INTERFACE_STRING ${PROJECT_NAME}_MAJOR_VERSION
)

# generate the version file for the config file
write_basic_package_version_file(
    "${CMAKE_CURRENT_BINARY_DIR}/${PROJECT_NAME}ConfigVersion.cmake"
    VERSION "${version}"
    COMPATIBILITY AnyNewerVersion
)

install(TARGETS ${PROJECT_NAME}
        EXPORT ${PROJECT_NAME}Targets
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
        RUNTIME DESTINATION ${CMAKE_INSTALL_LIBDIR}
        ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR})
set(PROTO_GENERATED_HEADERS ${PROTO_GENERATED_FILES})

list(FILTER PROTO_GENERATED_HEADERS INCLUDE REGEX .*\.h )

install(FILES ${PROTO_GENERATED_HEADERS}  DESTINATION ${ARMONIK_INSTALL_INCLUDE_DIR})
install(DIRECTORY ${HEADER_FILES_DIR}/ DESTINATION ${ARMONIK_INSTALL_INCLUDE_DIR} FILES_MATCHING PATTERN "*.h")

install(EXPORT ${PROJECT_NAME}Targets
        FILE ${PROJECT_NAME}Targets.cmake
        DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/${PROJECT_NAME}
)

configure_package_config_file(${CMAKE_CURRENT_SOURCE_DIR}/Config.cmake.in
  "${CMAKE_CURRENT_BINARY_DIR}/${PROJECT_NAME}Config.cmake"
  INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/${PROJECT_NAME}
  PATH_VARS CMAKE_INSTALL_PREFIX CMAKE_INSTALL_LIBDIR
)

install(FILES
          "${CMAKE_CURRENT_BINARY_DIR}/${PROJECT_NAME}Config.cmake"
          "${CMAKE_CURRENT_BINARY_DIR}/${PROJECT_NAME}ConfigVersion.cmake"
        DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/${PROJECT_NAME}
)
