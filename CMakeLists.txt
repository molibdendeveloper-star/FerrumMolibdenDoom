cmake_minimum_required(VERSION 3.20)
project(MolibdenDoom C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

find_package(SDL3 REQUIRED)

file(GLOB DOOM_SOURCES "src/doom/*.c")
file(GLOB PLATFORM_SOURCES "src/platform/*.c")

add_executable(molibdendoom
    ${DOOM_SOURCES}
    ${PLATFORM_SOURCES}
)

target_include_directories(molibdendoom PRIVATE 
    src/doom 
    src/platform
)

target_link_libraries(molibdendoom PRIVATE 
    SDL3::SDL3
    m
)

target_compile_definitions(molibdendoom PRIVATE 
    NORMALUNIX 
    LINUX
)

target_compile_options(molibdendoom PRIVATE
    -fno-strict-aliasing
    -fcommon
    -Wno-implicit-function-declaration
    -Wno-incompatible-pointer-types
    -Wno-int-conversion
)
