# CPack options
set(CPACK_THREADS 0)
set(CPACK_MONOLITHIC_INSTALL TRUE)

# Common options
set(CPACK_PACKAGE_NAME "libarmonik")
set(CPACK_PACKAGE_VENDOR "ANEO Consulting")
set(CPACK_PACKAGE_VERSION_MAJOR ${version_major})
set(CPACK_PACKAGE_VERSION_MINOR ${version_minor})
set(CPACK_PACKAGE_VERSION_PATCH ${version_patch})
set(CPACK_PACKAGE_DESCRIPTION "ArmoniK Api Libraries")
set(CPACK_PACKAGE_HOMEPAGE_URL "https://github.com/aneoconsulting/ArmoniK.Api")
set(CPACK_RESOURCE_FILE_LICENSE ${CMAKE_CURRENT_SOURCE_DIR}/LICENSE)

# Rpm options

# Deb options
