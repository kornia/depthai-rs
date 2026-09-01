cmake_minimum_required(VERSION 3.16)
project(depthai_c CXX)

# depthai-core's CMake config (CMAKE_PREFIX_PATH is passed by build.rs) supplies
# its include dirs, transitive deps and link flags.
find_package(depthai CONFIG REQUIRED)

add_library(depthai_c STATIC depthai_c.cpp depthai_nocv_stub.cpp)
target_compile_features(depthai_c PUBLIC cxx_std_17)
target_link_libraries(depthai_c PUBLIC depthai::core)
set_target_properties(depthai_c PROPERTIES POSITION_INDEPENDENT_CODE ON)

install(TARGETS depthai_c ARCHIVE DESTINATION lib)
