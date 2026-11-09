#
# CMakeLists.txt - Rust-on-FreeRTOS application, NUCLEO-F767ZI
#
# Author:  Sandesh Ghimire <sandesh@soccentric.com>
# Date:    2026-11-09
# Target:  STM32F767ZIT6
# SRS:     SRS-RFREERTOS-F767 (RFCON-001..004, RFGEN-001..005, RFBT-001)
#
# Build (inside the stm32-dev-rust-freertos container):
#   cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release
#   cmake --build build
# or simply: scripts/build.sh [clean]
#
# The C start-up, HAL and FreeRTOS kernel are compiled by CMake; the Rust
# application is a staticlib built by Cargo and linked after the C objects.
# Outputs in build/: rust-freertos-app.elf, .bin, .hex, .map and .img.
#

cmake_minimum_required(VERSION 3.22)

set(APP_VERSION "0.1.0")
set(APP_FIRMWARE_NAME "rust-freertos-app")
set(APP_SRS_ID "SRS-RFREERTOS-F767")
option(RF_FAIL_HEALTH "Force the pending-slot health check to fail (RFN-004)" OFF)

string(REPLACE "." ";" APP_VERSION_PARTS "${APP_VERSION}")
list(GET APP_VERSION_PARTS 0 APP_VERSION_MAJOR)
list(GET APP_VERSION_PARTS 1 APP_VERSION_MINOR)
list(GET APP_VERSION_PARTS 2 APP_VERSION_PATCH)

# Cross toolchain: the build runs in the container where arm-none-eabi-gcc
# is the only compiler. Set explicitly so a stray host compiler is never used.
set(CMAKE_SYSTEM_NAME     Generic)
set(CMAKE_SYSTEM_PROCESSOR arm)
set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY) # no OS to run a test exe in
set(CMAKE_EXPORT_COMPILE_COMMANDS ON) # compile_commands.json for clang-tidy
find_program(ARMCC arm-none-eabi-gcc REQUIRED)
find_program(ARMOBJCOPY arm-none-eabi-objcopy REQUIRED)
find_program(SIZE_TOOL arm-none-eabi-size REQUIRED)
set(CMAKE_C_COMPILER   ${ARMCC})
set(CMAKE_ASM_COMPILER ${ARMCC})
set(CMAKE_OBJCOPY      ${ARMOBJCOPY})

project(rust-freertos-app LANGUAGES C ASM)

# --- SDK location ----------------------------------------------------------
# STM32CubeF7 v1.17.4 is provided by the docker image (RFCON-002).
set(CUBE_ROOT "$ENV{STM32CUBE_F7}")
if(NOT CUBE_ROOT)
    set(CUBE_ROOT "/opt/st/STM32CubeF7")
endif()
if(NOT EXISTS "${CUBE_ROOT}/Drivers/STM32F7xx_HAL_Driver/Inc/stm32f7xx_hal.h")
    message(FATAL_ERROR
        "STM32CubeF7 not found at ${CUBE_ROOT}.\n"
        "Build through scripts/docker-dev.sh (see docker/README.md).")
endif()

# --- Rust toolchain --------------------------------------------------------
# cargo lives in /opt/cargo/bin in stm32-dev-rust-freertos; the hint also
# covers an interactive container where PATH was not inherited.
find_program(CARGO_EXECUTABLE NAMES cargo HINTS "$ENV{CARGO_HOME}/bin" /opt/cargo/bin)
if(NOT CARGO_EXECUTABLE)
    message(FATAL_ERROR
        "cargo was not found in PATH.\n"
        "Build inside the stm32-dev-rust-freertos image (scripts/docker-dev.sh),\n"
        "or install Rust 1.99.1 with the thumbv7em-none-eabihf target.")
endif()

set(RUST_CRATE_NAME  "rust_freertos_app")
set(RUST_TARGET      "thumbv7em-none-eabihf")
set(RUST_TARGET_DIR  "${CMAKE_BINARY_DIR}/rust")
set(RUST_STATICLIB
    "${RUST_TARGET_DIR}/${RUST_TARGET}/release/lib${RUST_CRATE_NAME}.a")

# RF_FAIL_HEALTH switches the Rust health check to fail through Cargo, so the
# CMake option and the Rust cfg can never disagree.
set(RUST_FEATURE_ARGS)
if(RF_FAIL_HEALTH)
    list(APPEND RUST_FEATURE_ARGS --features fail-health)
endif()

# --- Build metadata (RFGEN-002) -------------------------------------------
execute_process(COMMAND git -C ${CMAKE_CURRENT_SOURCE_DIR} rev-parse --short=12 HEAD
    OUTPUT_VARIABLE APP_GIT_COMMIT OUTPUT_STRIP_TRAILING_WHITESPACE ERROR_QUIET)
execute_process(COMMAND git -C ${CMAKE_CURRENT_SOURCE_DIR} rev-parse HEAD
    OUTPUT_VARIABLE APP_GIT_COMMIT_FULL OUTPUT_STRIP_TRAILING_WHITESPACE ERROR_QUIET)
if(NOT APP_GIT_COMMIT)
    set(APP_GIT_COMMIT "unknown")
    set(APP_GIT_COMMIT_FULL "")
endif()
if(DEFINED ENV{SOURCE_DATE_EPOCH} AND NOT "$ENV{SOURCE_DATE_EPOCH}" STREQUAL "")
    set(APP_BUILD_TIMESTAMP "$ENV{SOURCE_DATE_EPOCH}")
    execute_process(COMMAND date -u -d "@${APP_BUILD_TIMESTAMP}" "+%Y-%m-%d %H:%M:%S"
        OUTPUT_VARIABLE APP_BUILD_DATE OUTPUT_STRIP_TRAILING_WHITESPACE
        ERROR_QUIET)
    if(NOT APP_BUILD_DATE)
        string(TIMESTAMP APP_BUILD_DATE "%Y-%m-%d %H:%M:%S" UTC)
    endif()
else()
    string(TIMESTAMP APP_BUILD_DATE "%Y-%m-%d %H:%M:%S" UTC)
    string(TIMESTAMP APP_BUILD_TIMESTAMP "%s" UTC)
endif()

# --- Toolchain flags -------------------------------------------------------
set(CPU_FLAGS -mcpu=cortex-m7 -mthumb -mfpu=fpv5-d16 -mfloat-abi=hard)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

add_compile_options(${CPU_FLAGS} -Wall -Wextra -Werror -ffunction-sections -fdata-sections)
# No stdio and no libc heap (FreeRTOS heap_4 is static), so no syscall layer
# is linked.
add_link_options(${CPU_FLAGS} -Wl,--gc-sections -Wl,--cref
    -Wl,-Map=${CMAKE_BINARY_DIR}/${APP_FIRMWARE_NAME}.map
    --specs=nano.specs)

if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release)
endif()
set(CMAKE_C_FLAGS_RELEASE "-O2 -g3")
set(CMAKE_C_FLAGS_DEBUG   "-Og -g3")

add_compile_definitions(
    STM32F767xx
    USE_HAL_DRIVER
    HSE_VALUE=8000000U          # ST-LINK MCO feeds PH0 in bypass mode
    # RFBT-002: SystemInit points VTOR at the slot (BOOT also sets it).
    USER_VECT_TAB_ADDRESS
    VECT_TAB_OFFSET=0x0000C000U
    APP_VERSION_STR="${APP_VERSION}"
    APP_VERSION_MAJOR=${APP_VERSION_MAJOR}
    APP_VERSION_MINOR=${APP_VERSION_MINOR}
    APP_VERSION_PATCH=${APP_VERSION_PATCH}
    APP_FIRMWARE_NAME="${APP_FIRMWARE_NAME}"
    APP_SRS_ID="${APP_SRS_ID}"
    APP_GIT_COMMIT="${APP_GIT_COMMIT}"
    APP_BUILD_DATE="${APP_BUILD_DATE}"
    APP_BUILD_TIMESTAMP=${APP_BUILD_TIMESTAMP})

include_directories(src include board
    "${CUBE_ROOT}/Drivers/CMSIS/Include"
    "${CUBE_ROOT}/Drivers/CMSIS/Device/ST/STM32F7xx/Include"
    "${CUBE_ROOT}/Drivers/STM32F7xx_HAL_Driver/Inc")

# FreeRTOS heap size, configurable at build time (RFRT-001).  Rust tasks
# need more stack than the C demo, so the default is 32 KB, not 24 KB.
set(APP_HEAP_KB "32" CACHE STRING "FreeRTOS heap_4 size in KB")
math(EXPR APP_HEAP_BYTES "${APP_HEAP_KB} * 1024")

# --- Sources ---------------------------------------------------------------
# HAL modules used so far. The list grows with each SRS section.
set(HAL_SOURCES
    "${CUBE_ROOT}/Drivers/STM32F7xx_HAL_Driver/Src/stm32f7xx_hal.c"
    "${CUBE_ROOT}/Drivers/STM32F7xx_HAL_Driver/Src/stm32f7xx_hal_cortex.c"
    "${CUBE_ROOT}/Drivers/STM32F7xx_HAL_Driver/Src/stm32f7xx_hal_rcc.c"
    "${CUBE_ROOT}/Drivers/STM32F7xx_HAL_Driver/Src/stm32f7xx_hal_rcc_ex.c"
    "${CUBE_ROOT}/Drivers/STM32F7xx_HAL_Driver/Src/stm32f7xx_hal_gpio.c"
    "${CUBE_ROOT}/Drivers/STM32F7xx_HAL_Driver/Src/stm32f7xx_hal_pwr.c"
    "${CUBE_ROOT}/Drivers/STM32F7xx_HAL_Driver/Src/stm32f7xx_hal_pwr_ex.c"
    "${CUBE_ROOT}/Drivers/STM32F7xx_HAL_Driver/Src/stm32f7xx_hal_flash.c"
    "${CUBE_ROOT}/Drivers/STM32F7xx_HAL_Driver/Src/stm32f7xx_hal_flash_ex.c"
    "${CUBE_ROOT}/Drivers/STM32F7xx_HAL_Driver/Src/stm32f7xx_hal_dma.c"
    "${CUBE_ROOT}/Drivers/STM32F7xx_HAL_Driver/Src/stm32f7xx_hal_dma_ex.c"
    "${CUBE_ROOT}/Drivers/STM32F7xx_HAL_Driver/Src/stm32f7xx_hal_uart.c"
    "${CUBE_ROOT}/Drivers/STM32F7xx_HAL_Driver/Src/stm32f7xx_hal_uart_ex.c"
    "${CUBE_ROOT}/Drivers/STM32F7xx_HAL_Driver/Src/stm32f7xx_hal_iwdg.c")

set(PROJECT_SOURCES
    src/main.c
    src/platform.c
    src/clock.c
    src/hal_timebase.c
    src/cache_mpu.c
    src/crc32.c
    src/app_fault.c
    src/app_log.c
    src/reset_cause.c
    src/iwdg.c
    src/handover.c
    src/flash_confirm.c
    src/uart3.c
    src/rtos_hooks.c)

set(DEVICE_SOURCES
    "${CUBE_ROOT}/Drivers/CMSIS/Device/ST/STM32F7xx/Source/Templates/system_stm32f7xx.c"
    "${CUBE_ROOT}/Drivers/CMSIS/Device/ST/STM32F7xx/Source/Templates/gcc/startup_stm32f767xx.s")

# FreeRTOS V10.2.0 kernel + native GCC CM7 port as shipped (RFCON-002, RFRT-001)
set(FREERTOS_ROOT "${CUBE_ROOT}/Middlewares/Third_Party/FreeRTOS/Source")
include_directories(
    "${FREERTOS_ROOT}/include"
    "${FREERTOS_ROOT}/portable/GCC/ARM_CM7/r0p1")
set(FREERTOS_SOURCES
    "${FREERTOS_ROOT}/list.c"
    "${FREERTOS_ROOT}/queue.c"
    "${FREERTOS_ROOT}/tasks.c"
    "${FREERTOS_ROOT}/timers.c"
    "${FREERTOS_ROOT}/portable/GCC/ARM_CM7/r0p1/port.c"
    "${FREERTOS_ROOT}/portable/MemMang/heap_4.c")

# --- Rust staticlib --------------------------------------------------------
# One output file, so Ninja only calls Cargo when a Rust source, Cargo.toml,
# build.rs, VERSION or the lock file changes.  The glob is evaluated again
# by CMake when a file is added or removed.
file(GLOB RUST_SOURCES CONFIGURE_DEPENDS
    "${CMAKE_SOURCE_DIR}/src/*.rs"
    "${CMAKE_SOURCE_DIR}/src/hwlink/*.rs")

set(RUST_DEPENDS
    ${RUST_SOURCES}
    "${CMAKE_SOURCE_DIR}/Cargo.toml"
    "${CMAKE_SOURCE_DIR}/build.rs"
    "${CMAKE_SOURCE_DIR}/VERSION")
foreach(_opt Cargo.lock rust-toolchain.toml .cargo/config.toml)
    if(EXISTS "${CMAKE_SOURCE_DIR}/${_opt}")
        list(APPEND RUST_DEPENDS "${CMAKE_SOURCE_DIR}/${_opt}")
    endif()
endforeach()

add_custom_command(
    OUTPUT "${RUST_STATICLIB}"
    COMMAND ${CMAKE_COMMAND} -E env
            "CARGO_HOME=${CMAKE_SOURCE_DIR}/.cargo-home"
            "CARGO_TARGET_DIR=${RUST_TARGET_DIR}"
            "SOURCE_DATE_EPOCH=${APP_BUILD_TIMESTAMP}"
            "${CARGO_EXECUTABLE}" build --release
                --manifest-path "${CMAKE_SOURCE_DIR}/Cargo.toml"
                --target "${RUST_TARGET}"
                ${RUST_FEATURE_ARGS}
    DEPENDS ${RUST_DEPENDS}
    WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
    COMMENT "Building Rust staticlib (${RUST_TARGET}, release)"
    VERBATIM)
add_custom_target(rust_app ALL DEPENDS "${RUST_STATICLIB}")

# --- Firmware --------------------------------------------------------------
add_executable(app.elf ${PROJECT_SOURCES} ${HAL_SOURCES} ${FREERTOS_SOURCES}
    ${DEVICE_SOURCES})
set_target_properties(app.elf PROPERTIES
    OUTPUT_NAME "${APP_FIRMWARE_NAME}"
    SUFFIX ".elf")
target_compile_definitions(app.elf PRIVATE
    NDEBUG=1
    APP_RTOS_HEAP_SIZE=${APP_HEAP_BYTES})
if(RF_FAIL_HEALTH)
    target_compile_definitions(app.elf PRIVATE RF_FAIL_HEALTH=1)
endif()

# RFBT-001: vector table at 0x0800C000 (bootloader slot), at most 976 KB;
# the linker script fails the build otherwise.
target_link_options(app.elf PRIVATE
    "-T${CMAKE_SOURCE_DIR}/ld/app.ld")

# The Rust archive must appear after the C objects so that the references
# from main.c to rust_startup()/rust_*_task() pull the archive members in.
# libgcc is needed for the compiler helpers; the group handles any lingering
# ordering between the Rust archive and libgcc.
target_link_libraries(app.elf PRIVATE
    "-Wl,--start-group"
    "${RUST_STATICLIB}"
    gcc
    "-Wl,--end-group")
add_dependencies(app.elf rust_app)

# --- Static analysis (RFNFR-004) ------------------------------------------
# clang-tidy reuses the compile flags this very configure step produced, so
# it must run from a configured build tree:  make lint / make lint-strict.
find_program(CLANG_TIDY clang-tidy REQUIRED)
option(APP_LINT_STRICT "Treat any clang-tidy finding as an error (lint-strict)" OFF)
if(APP_LINT_STRICT)
    set(APP_LINT_FLAGS --warnings-as-errors=*)
endif()

# The target runs with the build dir as CWD, so make source paths absolute.
set(APP_LINT_SOURCES)
foreach(_src ${PROJECT_SOURCES})
    list(APPEND APP_LINT_SOURCES "${CMAKE_CURRENT_SOURCE_DIR}/${_src}")
endforeach()
math(EXPR MAX_FLASH_BYTES "128 * 1024")
math(EXPR MAX_RAM_BYTES "64 * 1024")

# clang-tidy runs the host clang but needs the newlib headers used by the
# arm-none-eabi toolchain in the container; otherwise <string.h> is missing.
set(APP_LINT_EXTRA_ARGS --extra-arg=-isystem
    --extra-arg=/usr/lib/arm-none-eabi/include)

add_custom_target(clang-tidy
    COMMAND ${CLANG_TIDY} -quiet -p ${CMAKE_BINARY_DIR} ${APP_LINT_FLAGS}
            ${APP_LINT_EXTRA_ARGS} ${APP_LINT_SOURCES}
    COMMENT "Running clang-tidy over the C platform sources"
    WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
    VERBATIM)

set(ELF "$<TARGET_FILE:app.elf>")
find_package(Python3 COMPONENTS Interpreter REQUIRED)

add_custom_command(TARGET app.elf POST_BUILD
    COMMAND ${CMAKE_OBJCOPY} -O binary  ${ELF}
            ${CMAKE_BINARY_DIR}/${APP_FIRMWARE_NAME}.bin
    COMMAND ${CMAKE_OBJCOPY} -O ihex    ${ELF}
            ${CMAKE_BINARY_DIR}/${APP_FIRMWARE_NAME}.hex
    COMMAND ${Python3_EXECUTABLE} ${CMAKE_SOURCE_DIR}/scripts/mkimage.py
            --input ${CMAKE_BINARY_DIR}/${APP_FIRMWARE_NAME}.bin
            --output ${CMAKE_BINARY_DIR}/${APP_FIRMWARE_NAME}.img
            --version=${APP_VERSION}
            --commit=${APP_GIT_COMMIT_FULL}
            --timestamp=${APP_BUILD_TIMESTAMP}
            --name=${APP_FIRMWARE_NAME}
    COMMAND ${SIZE_TOOL} $<TARGET_FILE:app.elf>
    COMMAND ${Python3_EXECUTABLE} ${CMAKE_SOURCE_DIR}/scripts/check_footprint.py
            --size-tool ${SIZE_TOOL}
            --max-flash ${MAX_FLASH_BYTES}
            --max-ram ${MAX_RAM_BYTES}
            $<TARGET_FILE:app.elf>
    BYPRODUCTS
        ${CMAKE_BINARY_DIR}/${APP_FIRMWARE_NAME}.bin
        ${CMAKE_BINARY_DIR}/${APP_FIRMWARE_NAME}.hex
        ${CMAKE_BINARY_DIR}/${APP_FIRMWARE_NAME}.img
        ${CMAKE_BINARY_DIR}/${APP_FIRMWARE_NAME}.map
    COMMENT "Generating ${APP_FIRMWARE_NAME}.bin/.hex/.img; checking footprint")
