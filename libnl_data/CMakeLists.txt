cmake_minimum_required(VERSION 3.16)
project(nl_data)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_FLAGS "-W -Wall -Wextra -Werror")

add_library(nl_data STATIC
        src/wireless.c
        src/common.c)

target_include_directories(nl_data INTERFACE  ${PROJECT_BINARY_DIR}/include)
target_include_directories(nl_data PRIVATE /usr/include/libnl3)
target_link_libraries(nl_data PRIVATE nl-3 nl-genl-3)

install(TARGETS nl_data DESTINATION lib)
install(FILES include/nl_data.h DESTINATION include)
