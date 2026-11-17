cmake_minimum_required(VERSION 3.20)
project(FerrumMolibdenDoom C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

find_package(SDL3 REQUIRED)

include(FetchContent)
FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG v0.5.0
)
FetchContent_MakeAvailable(Corrosion)

corrosion_import_crate(MANIFEST_PATH ferrum_doom/Cargo.toml)


file(GLOB DOOM_SOURCES "src/doom/*.c")
file(GLOB PLATFORM_SOURCES "src/platform/*.c")

add_executable(fmdoom
    ${DOOM_SOURCES}
    ${PLATFORM_SOURCES}
)

target_include_directories(fmdoom PRIVATE 
    src/doom 
    src/platform
)

target_link_libraries(fmdoom PRIVATE 
    SDL3::SDL3
    m
    ferrum_doom
)

target_compile_definitions(fmdoom PRIVATE 
    NORMALUNIX 
    LINUX
)

target_compile_options(fmdoom PRIVATE
    -fno-strict-aliasing
    -fcommon
    -Wno-implicit-function-declaration
    -Wno-incompatible-pointer-types
    -Wno-int-conversion
)
