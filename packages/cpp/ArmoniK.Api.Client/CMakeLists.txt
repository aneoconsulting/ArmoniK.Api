set(PROJECT_NAME ArmoniK.Api.Client)
set(NAMESPACE ArmoniK::Api::Client)
set(version 0.1.0)

set(PROTO_FILES 
    "submitter_service.proto"
    "applications_service.proto"
    "sessions_service.proto"
    # "tasks_service.proto" Name conflict with grpc generated files ListTasks and ListTasksRaw (grpc add the suffix RAW to all interfaces)
    "results_service.proto"
    "auth_service.proto"
    "partitions_service.proto"
    "events_service.proto"
    "versions_service.proto")

set(PROTO_DEPS
    "applications_common.proto"
    "session_status.proto"
    "auth_common.proto"
    "sessions_common.proto"
    "submitter_common.proto"
    "tasks_common.proto"
    "results_common.proto"
    "partitions_common.proto"
    "events_common.proto"
    "versions_common.proto"
    "sort_direction.proto"
    "objects.proto"
    "result_status.proto"
    "task_status.proto")

foreach(file ${PROTO_FILES} ${PROTO_DEPS})
    configure_file("${PROTO_FILES_DIR}/${file}" "${BUILD_DIR}/${PROJECT_NAME}/${file}" COPYONLY)
endforeach()
list(TRANSFORM PROTO_FILES PREPEND ${BUILD_DIR}/${PROJECT_NAME}/)

find_package(Protobuf REQUIRED)
find_package(gRPC CONFIG REQUIRED)
find_package(Threads)

SET(SOURCES_FILES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/source")
SET(HEADER_FILES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/header")

FILE(GLOB_RECURSE SRC_CLIENT_FILES ${SOURCES_FILES_DIR}/*.cpp)
FILE(GLOB_RECURSE HEADER_CLIENT_FILES ${HEADER_FILES_DIR}/*.h)


file(MAKE_DIRECTORY ${BUILD_DIR}/${PROJECT_NAME})

add_library(${PROJECT_NAME} STATIC ${PROTO_FILES} ${SRC_CLIENT_FILES} ${HEADER_CLIENT_FILES})

target_link_libraries(${PROJECT_NAME} PUBLIC protobuf::libprotobuf gRPC::grpc++_unsecure ArmoniK.Api.Common)

target_include_directories(${PROJECT_NAME}
                           PUBLIC
                           "$<BUILD_INTERFACE:${HEADER_FILES_DIR}>"
                           "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
)

set(PROTO_BINARY_DIR "${BUILD_DIR}/${PROJECT_NAME}/")
set(PROTO_IMPORT_DIRS "${PROTO_FILES_DIR}")

protobuf_generate(
    TARGET ${PROJECT_NAME}
    OUT_VAR PROTO_GENERATED_FILES
    IMPORT_DIRS ${BUILD_DIR}/${PROJECT_NAME}
    PROTOC_OUT_DIR "${PROTO_BINARY_DIR}")
set_source_files_properties(${PROTO_GENERATED_FILES} PROPERTIES SKIP_UNITY_BUILD_INCLUSION on)

get_target_property(grpc_cpp_plugin_location gRPC::grpc_cpp_plugin LOCATION)

protobuf_generate(
    TARGET ${PROJECT_NAME}
    OUT_VAR PROTO_GENERATED_FILES
    LANGUAGE grpc
    GENERATE_EXTENSIONS .grpc.pb.h .grpc.pb.cc
    PLUGIN "protoc-gen-grpc=${grpc_cpp_plugin_location}"
    # PLUGIN_OPTIONS "generate_mock_code=true"
    IMPORT_DIRS ${BUILD_DIR}/${PROJECT_NAME}
    PROTOC_OUT_DIR "${PROTO_BINARY_DIR}")

set_source_files_properties(${PROTO_GENERATED_FILES} PROPERTIES SKIP_UNITY_BUILD_INCLUSION on)

target_include_directories(${PROJECT_NAME}
                           PUBLIC
                           "$<BUILD_INTERFACE:${PROTO_BINARY_DIR}>"
                           "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>")

target_include_directories(${PROJECT_NAME}
                            PUBLIC
                            "$<BUILD_INTERFACE:${PROTO_BINARY_DIR}>"
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
        ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
        INCLUDES DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
        )

FILE(GLOB INCLUDE_FILES ${PROTO_BINARY_DIR}/*.h)
install(FILES ${INCLUDE_FILES} DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})

install(EXPORT ${PROJECT_NAME}Targets
        FILE ${PROJECT_NAME}Targets.cmake
        DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/${PROJECT_NAME}
)

configure_package_config_file(${CMAKE_CURRENT_SOURCE_DIR}/Config.cmake.in
  "${CMAKE_CURRENT_BINARY_DIR}/${PROJECT_NAME}Config.cmake"
  INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/${PROJECT_NAME}
)

install(FILES
          "${CMAKE_CURRENT_BINARY_DIR}/${PROJECT_NAME}Config.cmake"
          "${CMAKE_CURRENT_BINARY_DIR}/${PROJECT_NAME}ConfigVersion.cmake"
        DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/${PROJECT_NAME}
)
