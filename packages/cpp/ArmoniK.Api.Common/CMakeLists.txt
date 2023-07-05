# make cache variables for install destinations
include(GNUInstallDirs)
include(CMakePackageConfigHelpers)

set(PROJECT_NAME ArmoniK.Api.Common)
set(NAMESPACE ArmoniK::Api::Common)
set(ARMONIK_INSTALL_INCLUDE_DIR ${CMAKE_INSTALL_INCLUDEDIR}/armonik/common)

set(PROTO_FILES
    "objects.proto"
    "result_status.proto"
    "task_status.proto"
    "session_status.proto"
    "sort_direction.proto")

foreach(file ${PROTO_FILES})
    configure_file("${PROTO_FILES_DIR}/${file}" "${BUILD_DIR}/${PROJECT_NAME}/proto/${file}" COPYONLY)
endforeach()
list(TRANSFORM PROTO_FILES PREPEND ${BUILD_DIR}/${PROJECT_NAME}/proto/)

set(CMAKE_FIND_DEBUG_MODE FALSE)
# Trouver les packages requis
find_package(Protobuf REQUIRED)
find_package(gRPC CONFIG REQUIRED)
find_package(Threads)
set(CMAKE_FIND_DEBUG_MODE FALSE)

SET(SOURCES_FILES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/source")
SET(HEADER_FILES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/header")

FILE(GLOB_RECURSE SRC_CLIENT_FILES ${SOURCES_FILES_DIR}/*.cpp)
FILE(GLOB_RECURSE HEADER_CLIENT_FILES ${HEADER_FILES_DIR}/*.h)

file(MAKE_DIRECTORY ${BUILD_DIR}/${PROJECT_NAME})

add_library(${PROJECT_NAME} STATIC ${PROTO_FILES} ${SRC_CLIENT_FILES} ${HEADER_CLIENT_FILES})

target_link_libraries(${PROJECT_NAME} PUBLIC protobuf::libprotobuf gRPC::grpc++_unsecure)
set(PROTO_BINARY_DIR "${BUILD_DIR}/${PROJECT_NAME}/proto")
set(PROTO_IMPORT_DIRS "${PROTO_FILES_DIR}")

protobuf_generate(
    TARGET ${PROJECT_NAME}
    OUT_VAR PROTO_GENERATED_FILES
    APPEND_PATH "./"
    IMPORT_DIRS ${BUILD_DIR}/${PROJECT_NAME}/proto
    PROTOC_OUT_DIR "${PROTO_BINARY_DIR}")
set_source_files_properties(${PROTO_GENERATED_FILES} PROPERTIES SKIP_UNITY_BUILD_INCLUSION on)

get_target_property(grpc_cpp_plugin_location gRPC::grpc_cpp_plugin LOCATION)

target_include_directories(${PROJECT_NAME}
                           PUBLIC
                           "$<BUILD_INTERFACE:${PROTO_BINARY_DIR}>"
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

install(TARGETS ${PROJECT_NAME} EXPORT ${PROJECT_NAME}Targets)
set(PROTO_GENERATED_HEADERS ${PROTO_GENERATED_FILES})

list(FILTER PROTO_GENERATED_HEADERS INCLUDE REGEX .*\.h )

install(FILES ${PROTO_GENERATED_HEADERS}  DESTINATION ${ARMONIK_INSTALL_INCLUDE_DIR}/proto)
install(FILES ${HEADER_CLIENT_FILES} DESTINATION ${ARMONIK_INSTALL_INCLUDE_DIR})

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