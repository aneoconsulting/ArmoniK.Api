set(PROJECT_NAME ArmoniK.Api.Client)
set(ARMONIK_INSTALL_INCLUDE_DIR ${CMAKE_INSTALL_INCLUDEDIR}/armonik/client)
set(PROJECT_BUILD_DIR ${BUILD_DIR}/${PROJECT_NAME})

set(PROTO_FILES
    "submitter_service.proto"
    "applications_service.proto"
    "sessions_service.proto"
    "tasks_service.proto"
    "results_service.proto"
    "auth_service.proto"
    "partitions_service.proto"
    "events_service.proto"
    "versions_service.proto")

set(PROTO_MESSAGES
    "auth_common.proto"
    "sessions_common.proto"
    "submitter_common.proto"
    "tasks_common.proto"
    "results_common.proto"
    "partitions_common.proto"
    "events_common.proto"
    "versions_common.proto"
    "applications_common.proto")

list(TRANSFORM PROTO_FILES PREPEND "${PROTO_FILES_DIR}/")
list(TRANSFORM PROTO_MESSAGES PREPEND "${PROTO_FILES_DIR}/")

SET(SOURCES_FILES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/source")
SET(HEADER_FILES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/header")

FILE(GLOB_RECURSE SRC_CLIENT_FILES ${SOURCES_FILES_DIR}/*.cpp)
FILE(GLOB_RECURSE HEADER_CLIENT_FILES ${HEADER_FILES_DIR}/*.h)

# Trouver les packages requis
if (UNIX)
        find_package(Protobuf REQUIRED)
else()
        find_package(Protobuf CONFIG REQUIRED)
endif()
find_package(gRPC CONFIG REQUIRED)
find_package(Threads)

file(MAKE_DIRECTORY ${PROJECT_BUILD_DIR})

set(PROTO_BINARY_DIR "${PROJECT_BUILD_DIR}")
set(PROTO_IMPORT_DIRS "${PROTO_FILES_DIR}")

add_library(${PROJECT_NAME} ${PROTO_GENERATED_FILES} ${SRC_CLIENT_FILES} ${HEADER_CLIENT_FILES})

protobuf_generate(
    TARGET ${PROJECT_NAME}
    PROTOS ${PROTO_MESSAGES} ${PROTO_FILES}
    OUT_VAR PROTO_GENERATED_MESSAGES
    APPEND_PATH "./"
    IMPORT_DIRS ${PROTO_IMPORT_DIRS}
    PROTOC_OUT_DIR "${PROTO_BINARY_DIR}")
set_source_files_properties(${PROTO_GENERATED_MESSAGES} PROPERTIES SKIP_UNITY_BUILD_INCLUSION on)

get_target_property(grpc_cpp_plugin_location gRPC::grpc_cpp_plugin LOCATION)

protobuf_generate(
    TARGET ${PROJECT_NAME}
    OUT_VAR PROTO_GENERATED_FILES
    PROTOS ${PROTO_FILES}
    APPEND_PATH "./"
    LANGUAGE grpc
    GENERATE_EXTENSIONS .grpc.pb.h .grpc.pb.cc
    PLUGIN "protoc-gen-grpc=${grpc_cpp_plugin_location}"
    # PLUGIN_OPTIONS "generate_mock_code=true"
    IMPORT_DIRS ${PROTO_IMPORT_DIRS}
    PROTOC_OUT_DIR "${PROTO_BINARY_DIR}")

set_source_files_properties(${PROTO_GENERATED_FILES} PROPERTIES SKIP_UNITY_BUILD_INCLUSION on)
list(APPEND PROTO_GENERATED_FILES ${PROTO_GENERATED_MESSAGES})

target_link_libraries(${PROJECT_NAME} PUBLIC protobuf::libprotobuf gRPC::grpc++_unsecure ArmoniK.Api.Common)

setup_options(${PROJECT_NAME})

target_include_directories(${PROJECT_NAME}
                           PUBLIC
                           "$<BUILD_INTERFACE:${PROJECT_BUILD_DIR}>"
                           "$<BUILD_INTERFACE:${HEADER_FILES_DIR}>"
                           "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
)

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

install(FILES ${PROTO_GENERATED_HEADERS} DESTINATION ${ARMONIK_INSTALL_INCLUDE_DIR})
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
