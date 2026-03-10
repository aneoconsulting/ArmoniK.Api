# make cache variables for install destinations
include(GNUInstallDirs)
include(CMakePackageConfigHelpers)

set(PROJECT_NAME ArmoniK.Api.Common)
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

# Trouver les packages requis
if (UNIX)
    find_package(Protobuf REQUIRED)
else()
    find_package(Protobuf CONFIG REQUIRED)
endif()
find_package(gRPC CONFIG REQUIRED)
find_package(Threads)
find_package(absl REQUIRED)

# Check for Red Hat-based systems and static linking
file(READ /etc/os-release OS_RELEASE_CONTENT)
if(OS_RELEASE_CONTENT MATCHES "^(ID=|ID_LIKE=).*rhel|fedora" AND STATIC_LINK)
    # On Red Hat systems with static linking, we need to find the utf8_range and utf8_validity libraries.
    find_package(utf8_range REQUIRED)
    find_library(UTF8_VALIDITY_LIB
        NAMES utf8_validity
        PATHS /usr/lib
        NO_DEFAULT_PATH
    )
    if(NOT UTF8_VALIDITY_LIB)
        message(FATAL_ERROR "utf8_validity library not found!")
    else()
        message(STATUS "Found utf8_validity library: ${UTF8_VALIDITY_LIB}")
    endif()
endif()

include(FetchContent)

FETCHCONTENT_DECLARE(
        fmt
        URL "${SRC_FMT}"
        TIMEOUT 180
)

FetchContent_GetProperties(fmt)
if(NOT fmt_POPULATED)
    FetchContent_Populate(fmt)
endif()

FETCHCONTENT_DECLARE(
        simdjson
        URL "${SRC_SIMDJSON}"
        TIMEOUT 180
)

FetchContent_GetProperties(simdjson)
if(NOT simdjson_POPULATED)
    FetchContent_Populate(simdjson)
endif()

SET(SOURCES_FILES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/source")
SET(HEADER_FILES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/header")

FILE(GLOB_RECURSE SRC_CLIENT_FILES ${SOURCES_FILES_DIR}/*.cpp)
FILE(GLOB_RECURSE HEADER_CLIENT_FILES ${HEADER_FILES_DIR}/*.h)

file(MAKE_DIRECTORY ${PROJECT_BUILD_DIR})

add_library(${PROJECT_NAME} ${PROTO_GENERATED_FILES} ${SRC_CLIENT_FILES} ${HEADER_CLIENT_FILES} ${simdjson_SOURCE_DIR}/singleheader/simdjson.cpp ${simdjson_SOURCE_DIR}/singleheader/simdjson.h)

target_link_libraries(${PROJECT_NAME} PUBLIC protobuf::libprotobuf gRPC::grpc++)
target_compile_definitions(${PROJECT_NAME} PUBLIC FMT_HEADER_ONLY=1)
set_property(TARGET ${PROJECT_NAME} PROPERTY CXX_STANDARD 14)

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
                           "$<BUILD_INTERFACE:${fmt_SOURCE_DIR}/include>"
                           "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>")

# Check if the system is Red Hat for linking Abseil
if(OS_RELEASE_CONTENT MATCHES "^(ID=|ID_LIKE=).*rhel|fedora")
    message(STATUS "Detected Red Hat system.")
    if(STATIC_LINK)
        message(STATUS "Static linking is enabled. Linking Abseil libraries.")

        target_link_libraries(${PROJECT_NAME} PUBLIC
            utf8_range::utf8_range
            ${UTF8_VALIDITY_LIB}

            absl::log
            absl::check
            absl::log_internal_check_op
            absl::log_internal_message
            absl::log_internal_format
            absl::log_internal_proto
            absl::log_internal_globals
            absl::log_internal_conditions
            absl::raw_logging_internal

            absl::strings
            absl::string_view
            absl::throw_delegate
            absl::base
        )
    else()
        message(STATUS "Static linking is not enabled. Skipping Abseil libraries.")
    endif()
else()
    message(STATUS "Not a Red Hat system. Skipping Abseil libraries.")
endif()

set_property(TARGET ${PROJECT_NAME} PROPERTY VERSION ${version})
set_property(TARGET ${PROJECT_NAME} PROPERTY SOVERSION ${version_major})
set_property(TARGET ${PROJECT_NAME} PROPERTY
    INTERFACE_${PROJECT_NAME}_MAJOR_VERSION ${version_major})
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
