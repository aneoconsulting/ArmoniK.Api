set(PROJECT_NAME ArmoniK.Api.Worker.Tests)

# Trouver les packages requis
find_package(Protobuf REQUIRED)
find_package(gRPC CONFIG REQUIRED)
find_package(Threads)

SET(SOURCES_FILES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/source")
SET(HEADER_FILES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/header")


FILE(GLOB_RECURSE SRC_CLIENT_FILES ${SOURCES_FILES_DIR}/*.cpp)
FILE(GLOB_RECURSE HEADER_CLIENT_FILES ${HEADER_FILES_DIR}/*.h)

add_executable(${PROJECT_NAME} ${SRC_CLIENT_FILES} ${HEADER_CLIENT_FILES})

set_property(TARGET ${PROJECT_NAME} PROPERTY CXX_STANDARD 14)

target_link_libraries(${PROJECT_NAME} PUBLIC protobuf::libprotobuf gRPC::grpc++_unsecure ArmoniK.Api.Worker ArmoniK.Api.Common)
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
