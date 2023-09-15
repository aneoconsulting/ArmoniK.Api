# ArmoniK Api Cpp

## Build requirements
- cmake 3.22+
- C++ compiler with C++14 support
- grpc 1.54 - 1.56.2
- protobuf
- fmt 10.1.0 (https://github.com/fmtlib/fmt/archive/refs/tags/10.1.0.tar.gz)
- simdjson 3.2.2 (https://github.com/simdjson/simdjson/archive/refs/tags/v3.2.2.tar.gz)
- gtest (https://github.com/google/googletest/archive/03597a01ee50ed33e9dfd640b249b4be3799d395.zip) (if BUILD_TEST=ON)

## How to build
```shell
cmake -S . -B out
cmake --build out
cmake --install out
```
