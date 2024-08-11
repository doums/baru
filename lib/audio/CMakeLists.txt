cmake_minimum_required(VERSION 3.28)
project(audio C)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(CMAKE_C_STANDARD 11)
set(CMAKE_C_FLAGS "-W -Wall -Wextra -Werror")

include(FindPkgConfig)

add_library(audio STATIC src/audio.c)

if (PkgConfig_FOUND)
    pkg_check_modules(LIBPULSE libpulse>=11)
endif ()

if (NOT LIBPULSE_LINK_LIBRARIES)
    message(FATAL_ERROR "libpulse not found")
endif ()

target_include_directories(audio INTERFACE ${PROJECT_BINARY_DIR}/include)
target_link_libraries(audio PRIVATE ${LIBPULSE_LINK_LIBRARIES})
target_include_directories(audio PUBLIC ${LIBPULSE_INCLUDE_DIRS})

install(TARGETS audio DESTINATION lib)
install(FILES include/audio.h DESTINATION include)
