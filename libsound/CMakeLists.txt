cmake_minimum_required(VERSION 3.17)
project(sound C)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(CMAKE_C_STANDARD 11)
set(CMAKE_C_FLAGS "-W -Wall -Wextra -Werror")

include(FindPkgConfig)

add_library(sound STATIC src/sound.c)

if (PkgConfig_FOUND)
    pkg_check_modules(LIBPULSE libpulse>=11)
endif ()

if (NOT LIBPULSE_LINK_LIBRARIES)
    message(FATAL_ERROR "libpulse not found")
endif ()

target_include_directories(sound INTERFACE ${PROJECT_BINARY_DIR}/include)
target_link_libraries(sound PRIVATE ${LIBPULSE_LINK_LIBRARIES})
target_include_directories(sound PUBLIC ${LIBPULSE_INCLUDE_DIRS})

install(TARGETS sound DESTINATION lib)
install(FILES include/sound.h DESTINATION include)
