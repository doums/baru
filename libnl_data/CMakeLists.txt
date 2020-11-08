cmake_minimum_required(VERSION 3.17)
project(nl_data)
include(FindPkgConfig)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(CMAKE_C_STANDARD 11)
set(CMAKE_C_FLAGS "-W -Wall -Wextra -Werror")

add_library(nl_data STATIC
        src/wireless.c
        src/wired.c
        src/common.c)

if (PkgConfig_FOUND)
    pkg_check_modules(LIBNL libnl-3.0>=3.1 libnl-route-3.0>=3.1 libnl-genl-3.0>=3.1)
endif ()

if (NOT LIBNL_LINK_LIBRARIES)
    message(FATAL_ERROR "libnl-3 not found")
endif ()

target_include_directories(nl_data INTERFACE ${PROJECT_BINARY_DIR}/include)
target_link_libraries(nl_data PRIVATE ${LIBNL_LINK_LIBRARIES})
target_include_directories(nl_data PRIVATE ${LIBNL_INCLUDE_DIRS})

install(TARGETS nl_data DESTINATION lib)
install(FILES include/nl_data.h DESTINATION include)
