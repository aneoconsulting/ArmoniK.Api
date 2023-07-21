set(PROJECT_NAME ArmoniK.Api.Tests)
set(NAMESPACE ArmoniK::Api::Tests)

# Trouver les packages requis
find_package(Protobuf REQUIRED)
find_package(gRPC CONFIG REQUIRED)
find_package(Threads)

SET(SOURCES_FILES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/source")
SET(HEADER_FILES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/header")

FILE(GLOB_RECURSE SRC_CLIENT_FILES ${SOURCES_FILES_DIR}/*.cpp)
FILE(GLOB_RECURSE HEADER_CLIENT_FILES ${HEADER_FILES_DIR}/*.h)

#file(MAKE_DIRECTORY ${BUILD_DIR}/${PROJECT_NAME})

add_executable(${PROJECT_NAME} ${SRC_CLIENT_FILES} ${HEADER_CLIENT_FILES})

set_property(TARGET ${PROJECT_NAME} PROPERTY CXX_STANDARD 17)

target_link_libraries(${PROJECT_NAME} PUBLIC protobuf::libprotobuf gRPC::grpc++_unsecure ArmoniK.Api.Client ArmoniK.Api.Common GTest::gtest_main GTest::gmock_main)

if(MSVC)
    target_compile_options(${PROJECT_NAME} PRIVATE /W4)
else()
    target_compile_options(${PROJECT_NAME} PRIVATE -Wall -Wextra)
endif()

set(PROTO_BINARY_DIR "${BUILD_DIR}/${PROJECT_NAME}/")
set(PROTO_IMPORT_DIRS "${PROTO_FILES_DIR}")

target_compile_definitions(${PROJECT_NAME} PUBLIC API_TEST_NAMESPACE=${NAMESPACE})

target_include_directories(${PROJECT_NAME}
                           PUBLIC
                           "$<BUILD_INTERFACE:${HEADER_FILES_DIR}>"
                           "$<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>")

set_property(TARGET ${PROJECT_NAME} PROPERTY VERSION ${version})
set_property(TARGET ${PROJECT_NAME} PROPERTY SOVERSION 0)
set_property(TARGET ${PROJECT_NAME} PROPERTY
    INTERFACE_${PROJECT_NAME}_MAJOR_VERSION 0)
set_property(TARGET ${PROJECT_NAME} APPEND PROPERTY
    COMPATIBLE_INTERFACE_STRING ${PROJECT_NAME}_MAJOR_VERSION
)

# gTest support
include(FetchContent)
FetchContent_Declare(
    googletest
    URL https://github.com/google/googletest/archive/03597a01ee50ed33e9dfd640b249b4be3799d395.zip
    DOWNLOAD_EXTRACT_TIMESTAMP TRUE
)
# For Windows: Prevent overriding the parent project's compiler/linker settings
set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)
FetchContent_MakeAvailable(googletest)

include(GoogleTest)
gtest_discover_tests(${PROJECT_NAME})

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

#FILE(GLOB INCLUDE_FILES ${PROTO_BINARY_DIR}/*.h)
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
