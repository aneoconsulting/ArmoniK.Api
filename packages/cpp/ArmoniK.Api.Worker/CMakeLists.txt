set(PROJECT_NAME ArmoniK.Api.Worker)
set(ARMONIK_INSTALL_INCLUDE_DIR ${CMAKE_INSTALL_INCLUDEDIR}/armonik/worker)
set(PROJECT_BUILD_DIR ${BUILD_DIR}/${PROJECT_NAME})

set(PROTO_FILES 
    "worker_service.proto"
    "agent_service.proto")

set(PROTO_MESSAGES 
    "agent_common.proto"
    "worker_common.proto")

list(TRANSFORM PROTO_FILES PREPEND ${PROTO_FILES_DIR}/)
list(TRANSFORM PROTO_MESSAGES PREPEND ${PROTO_FILES_DIR}/)


SET(SOURCES_FILES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/source")
SET(HEADER_FILES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/header")

FILE(GLOB_RECURSE SRC_CLIENT_FILES ${SOURCES_FILES_DIR}/*.cpp)
FILE(GLOB_RECURSE HEADER_CLIENT_FILES ${HEADER_FILES_DIR}/*.h)

file(MAKE_DIRECTORY ${PROJECT_BUILD_DIR}/proto)

add_library(${PROJECT_NAME} ${PROTO_GENERATED_FILES} ${SRC_CLIENT_FILES} ${HEADER_CLIENT_FILES})

# Trouver les packages requis
if (UNIX)
        find_package(Protobuf REQUIRED)
else()
        find_package(Protobuf CONFIG REQUIRED)
endif()
find_package(gRPC CONFIG REQUIRED)
find_package(Threads)

target_link_libraries(${PROJECT_NAME} PUBLIC protobuf::libprotobuf gRPC::grpc++ ArmoniK.Api.Common ${PROTO_TARGET})
target_link_options(${PROJECT_NAME} PUBLIC "-Wl,--copy-dt-needed-entries")

target_include_directories(${PROJECT_NAME}
    PUBLIC
    "$<BUILD_INTERFACE:${BUILD_DIR}/ArmoniK.Api.Common>"
    "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>"
    "$<TARGET_PROPERTY:ArmoniK.Api.Common,INTERFACE_INCLUDE_DIRECTORIES>"
)

setup_options(${PROJECT_NAME})

set(PROTO_BINARY_DIR "${PROJECT_BUILD_DIR}")
set(PROTO_IMPORT_DIRS "${PROTO_FILES_DIR}")

protobuf_generate(
    TARGET ${PROJECT_NAME}
    PROTOS ${PROTO_FILES} ${PROTO_MESSAGES}
    OUT_VAR PROTO_GENERATED_MESSAGES
    APPEND_PATH "./"
    IMPORT_DIRS ${PROTO_IMPORT_DIRS}
    PROTOC_OUT_DIR "${PROTO_BINARY_DIR}")

set_source_files_properties(${PROTO_GENERATED_MESSAGES} PROPERTIES SKIP_UNITY_BUILD_INCLUSION on)

get_target_property(grpc_cpp_plugin_location gRPC::grpc_cpp_plugin LOCATION)

protobuf_generate(
    TARGET ${PROJECT_NAME}
    PROTOS ${PROTO_FILES}
    OUT_VAR PROTO_GENERATED_FILES
    APPEND_PATH "./"
    LANGUAGE grpc
    GENERATE_EXTENSIONS .grpc.pb.h .grpc.pb.cc
    PLUGIN "protoc-gen-grpc=${grpc_cpp_plugin_location}"
    # PLUGIN_OPTIONS "generate_mock_code=true"
    IMPORT_DIRS ${PROTO_IMPORT_DIRS}
    PROTOC_OUT_DIR "${PROTO_BINARY_DIR}")

set_source_files_properties(${PROTO_GENERATED_FILES} PROPERTIES SKIP_UNITY_BUILD_INCLUSION on)
list(APPEND PROTO_GENERATED_FILES ${PROTO_GENERATED_MESSAGES})

target_include_directories(${PROJECT_NAME}
                           PUBLIC
                           "$<BUILD_INTERFACE:${HEADER_FILES_DIR}>"
                           "$<BUILD_INTERFACE:${PROJECT_BUILD_DIR}>"
                           "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>")

set_property(TARGET ${PROJECT_NAME} PROPERTY VERSION ${version})
set_property(TARGET ${PROJECT_NAME} PROPERTY SOVERSION ${version_major})
set_property(TARGET ${PROJECT_NAME} PROPERTY
        INTERFACE_${PROJECT_NAME}_MAJOR_VERSION ${version_major})
set_property(TARGET ${PROJECT_NAME} APPEND PROPERTY
        COMPATIBLE_INTERFACE_STRING ${PROJECT_NAME}_MAJOR_VERSION
        )
set_property(TARGET ${PROJECT_NAME} PROPERTY CXX_STANDARD 14)

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
