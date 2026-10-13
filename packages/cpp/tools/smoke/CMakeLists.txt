cmake_minimum_required(VERSION 3.22)
project(ArmoniK.Api.Smoke CXX)

# Builds against the installed client package to check that it links and runs
find_package(ArmoniK.Api.Client CONFIG REQUIRED)

add_executable(smoke main.cpp)
target_link_libraries(smoke PRIVATE ArmoniK.Api.Client)
set_property(TARGET smoke PROPERTY CXX_STANDARD 14)
