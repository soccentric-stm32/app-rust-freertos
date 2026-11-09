# Docker Development Environment

| | |
|---|---|
| **Author** | Sandesh Ghimire <sandesh@soccentric.com> |
| **Date** | 2026-11-09 |
| **Board** | NUCLEO-F767ZI |

The image `stm32-dev-rust-freertos:latest` extends the shared
`stm32-dev:latest` image of the C projects, so CMake/Ninja, OpenOCD, GDB,
picocom, `arm-none-eabi-gcc`, `arm-none-eabi-objcopy` and the
`scripts/docker-dev.sh` commands (flash, debug, serial, probe) work exactly as
there. It adds Rust 1.99.1 with the `thumbv7em-none-eabihf` target, `clippy`
and `rustfmt`, in `/opt/rustup`.

The C start-up code, STM32 HAL, FreeRTOS kernel and platform layer are built
by CMake. CMake calls Cargo for the Rust staticlib and links it into the same
ELF, so one image contains the whole mixed C/Rust firmware.

| File | Role |
|---|---|
| [Dockerfile](Dockerfile) | The Rust layer on top of `stm32-dev` (build arg `RUST_VERSION`) |
| [Dockerfile.base](Dockerfile.base) | Identical copy of the validation firmware's Dockerfile, used to build `stm32-dev` when it is not on the machine yet |

`scripts/docker-dev.sh build` builds the base image first if it is missing,
then this one. The full description of the base image is in the validation
firmware's
[docker/README.md](https://github.com/soccentric-stm32/validation-firmware/blob/HEAD/docker/README.md).

| Variable | Default |
|---|---|
| `STM32_IMAGE` | `stm32-dev-rust-freertos:latest` |
| `BASE_IMAGE` | `stm32-dev:latest` |

## Quick start

```bash
scripts/docker-dev.sh build      # build the base image (if needed) and this image
scripts/build.sh                 # configure + build the C platform and Rust app
make flash                       # upload build/rust-freertos-app.img through BOOT
make serial                      # ST-LINK VCP console, 115200 8N1
make fmt                         # cargo fmt in the container
```

Run `make help` for all targets and `scripts/docker-dev.sh --help` for the
container commands.
