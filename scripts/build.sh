#!/usr/bin/env bash
#
# build.sh - Configure and build the Rust-on-FreeRTOS application inside
#            the stm32-dev-rust-freertos container
#
# Author:  Sandesh Ghimire <sandesh@soccentric.com>
# Usage:   scripts/build.sh            configure (if needed) + build
#          scripts/build.sh clean      rm -rf build target, then configure + build
#
# Outputs: build/rust-freertos-app.elf, .bin, .hex, .map and .img for BOOT.
#
# CMake builds the C platform and FreeRTOS kernel, then calls Cargo for the
# Rust staticlib and links it after the C objects.  SOURCE_DATE_EPOCH is
# passed into the container so the build metadata and image header are
# reproducible.
#

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
DEV="$ROOT/scripts/docker-dev.sh"
cd "$ROOT"

if [[ "${1:-}" == "clean" ]]; then
    rm -rf build target
fi

STAMP="${SOURCE_DATE_EPOCH:-$(date +%s)}"
export SOURCE_DATE_EPOCH="$STAMP"

"$DEV" run env SOURCE_DATE_EPOCH="$STAMP" \
    cmake -S /work -B /work/build -G Ninja \
    -DCMAKE_BUILD_TYPE="${CMAKE_BUILD_TYPE:-Release}"
"$DEV" run env SOURCE_DATE_EPOCH="$STAMP" \
    cmake --build /work/build
