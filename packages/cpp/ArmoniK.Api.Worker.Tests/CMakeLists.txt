set(PROJECT_NAME ArmoniK.Api.Worker.Tests)

# Trouver les packages requis
find_package(Protobuf REQUIRED)
find_package(gRPC CONFIG REQUIRED)
find_package(Threads)
find_package(absl REQUIRED)

option(STATIC_LINK "Link statically" ON)

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

SET(SOURCES_FILES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/source")
SET(HEADER_FILES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/header")

FILE(GLOB_RECURSE SRC_WORKER_FILES ${SOURCES_FILES_DIR}/*.cpp)
FILE(GLOB_RECURSE HEADER_WORKER_FILES ${HEADER_FILES_DIR}/*.h)

add_executable(${PROJECT_NAME} ${SRC_WORKER_FILES} ${HEADER_WORKER_FILES})

set_property(TARGET ${PROJECT_NAME} PROPERTY CXX_STANDARD 14)

target_link_options(${PROJECT_NAME} PRIVATE
    -Wl,--start-group
)

target_link_libraries(${PROJECT_NAME} PUBLIC
    gRPC::grpc++
    protobuf::libprotobuf

    ArmoniK.Api.Worker
    ArmoniK.Api.Common
)

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

set(PROTO_BINARY_DIR "${BUILD_DIR}/${PROJECT_NAME}/")
set(PROTO_IMPORT_DIRS "${PROTO_FILES_DIR}")

target_include_directories(${PROJECT_NAME}
                           PUBLIC
                           "$<BUILD_INTERFACE:${HEADER_FILES_DIR}>"
                           "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>")

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
        ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
        INCLUDES DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
        )
