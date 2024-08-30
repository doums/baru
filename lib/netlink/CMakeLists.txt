cmake_minimum_required(VERSION 3.28)
project(netlink)
include(FindPkgConfig)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(CMAKE_C_STANDARD 23)
set(CMAKE_C_FLAGS "-W -Wall -Wextra -Werror")

add_library(netlink STATIC
        src/wireless.c
        src/wired.c
        src/common.c)

if (PkgConfig_FOUND)
    pkg_check_modules(LIBNL libnl-3.0>=3.1 libnl-route-3.0>=3.1 libnl-genl-3.0>=3.1)
endif ()

if (NOT LIBNL_LINK_LIBRARIES)
    message(FATAL_ERROR "libnl-3 not found")
endif ()

target_include_directories(netlink INTERFACE ${PROJECT_BINARY_DIR}/include)
target_link_libraries(netlink PRIVATE ${LIBNL_LINK_LIBRARIES})
target_include_directories(netlink PRIVATE ${LIBNL_INCLUDE_DIRS})

install(TARGETS netlink DESTINATION lib)
install(FILES include/netlink.h DESTINATION include)
