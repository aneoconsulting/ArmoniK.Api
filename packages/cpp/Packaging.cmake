# CPack options
set(CPACK_THREADS 0)
set(CPACK_MONOLITHIC_INSTALL TRUE)

# Common options
set(CPACK_PACKAGE_NAME "libarmonik")
set(CPACK_PACKAGE_VENDOR "ANEO Consulting")
set(CPACK_PACKAGE_VERSION_MAJOR ${version_major})
set(CPACK_PACKAGE_VERSION_MINOR ${version_minor})
set(CPACK_PACKAGE_VERSION_PATCH ${version_patch})
set(CPACK_PACKAGE_DESCRIPTION_FILE "${CMAKE_CURRENT_SOURCE_DIR}/tools/packaging/common/DESCRIPTION")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "ArmoniK API Libraries")
set(CPACK_PACKAGE_HOMEPAGE_URL "https://github.com/aneoconsulting/ArmoniK.Api")
set(CPACK_RESOURCE_FILE_LICENSE "${CMAKE_CURRENT_SOURCE_DIR}/tools/packaging/common/LICENSE")
set(CPACK_PACKAGE_CONTACT "armonik-support@aneo.fr")

# Rpm options
set(CPACK_RPM_PACKAGE_LICENSE "Apache 2.0")
set(CPACK_RPM_PACKAGE_GROUP "Development Tools")
set(CPACK_RPM_CHANGELOG_FILE "${CMAKE_CURRENT_SOURCE_DIR}/tools/packaging/common/CHANGELOG")

# Deb options
if("DEB" IN_LIST CPACK_GENERATOR)
    set(CPACK_DEBIAN_PACKAGE_GENERATE_SHLIBS ON)
    set(CPACK_DEBIAN_PACKAGE_RELEASE 1)
    file(READ "${CMAKE_CURRENT_SOURCE_DIR}/tools/packaging/debian/control" DEBIAN_CONTROL_FILE)
    if(${DEBIAN_CONTROL_FILE} MATCHES "Build-Depends: ([^\r\n]*)")
        set(DEBIAN_PACKAGE_BUILDS_DEPENDS "${CMAKE_MATCH_1}")
    else ()
        message(FATAL_ERROR "Build dependencies not found in control file")
    endif()
    if(${DEBIAN_CONTROL_FILE} MATCHES "[^\-]Depends: ([^\r\n]*)")
        set(CPACK_DEBIAN_PACKAGE_DEPENDS "${CMAKE_MATCH_1}")
    else ()
        message(FATAL_ERROR "Runtime dependencies not found in control file")
    endif()
endif()