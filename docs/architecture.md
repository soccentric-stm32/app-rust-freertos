# Architecture

The firmware is one image built from two languages:

- **Rust** (`src/`) owns the application: the four FreeRTOS tasks, the HWLINK
  protocol and the shared application state.
- **C** (`platform/`) owns everything the application must not reimplement:
  start-up, STM32 HAL drivers, the FreeRTOS kernel hooks and the bootloader
  flash/boot-domain access.

The Rust staticlib is linked after the C objects. `platform/main.c` brings up
the HAL, clocks, GPIO, USART6, USART3 and IWDG, calls `rust_startup()`, then
creates the task entries exported by the Rust archive and starts the
scheduler.

## Layers

```
┌──────────────────────────────────────────────────────────────────────┐
│ src/app/          health │ comms │ button │ led                      │
│                   ipc (shared state + queues), fault, identity       │
├──────────────────────────────────────────────────────────────────────┤
│ src/protocol/     hwlink: frame (COBS/CRC-32) + fb (FlatBuffers)     │
├──────────────────────────────────────────────────────────────────────┤
│ src/platform/     plat (FFI), board, boot, build_info                │
├──────────────────────────────────────────────────────────────────────┤
│ platform/         platform.c/.h + boot/ drivers/ fault/ log/ rtos/   │
├──────────────────────────────────────────────────────────────────────┤
│ STM32CubeF7       HAL, CMSIS device start-up, FreeRTOS kernel        │
└──────────────────────────────────────────────────────────────────────┘
```

Rules of thumb:

- Application logic belongs in `src/app/`; it must stay `no_std` and must not
  allocate.
- A new hardware capability is a C driver under `platform/drivers/` and a safe
  wrapper in `src/platform/plat.rs` (keep the declarations next to
  `platform/platform.h`).
- A new wire message is added to `protocol/hwlink.fbs`, then to
  `src/protocol/hwlink/mod.rs` and the codec in `fb.rs`.
- Board pins and FreeRTOS/HAL configuration stay in `board/`.
- Linker layout and the bootloader contract stay in `ld/` and
  `platform/boot/boot_contract.h` respectively.

## Boot flow

1. BOOT validates the image in slot A (`0x0800C000`) or slot B and jumps into
   it. The vector table is linked for the slot (`ld/app.ld`).
2. `platform/main.c` initialises clocks, cache/MPU, GPIO, USART6, USART3 and
   IWDG, decodes the hand-over record and calls `rust_startup()`.
3. `rust_startup()` publishes the reset cause and hand-over state, logs a
   start line, checks VTOR and publishes the initial status.
4. The scheduler starts the four tasks. Health is the only task that feeds the
   watchdog; after a successful check on a trial slot it writes the CONFIRMED
   record through the C platform layer.
5. A fault, FreeRTOS assert, stack overflow or Rust panic is recorded in the
   no-init fault window by `platform/fault/app_fault.c` and ends in
   `plat_fatal()`; the next boot reports the record.

## Task model

| Task   | Priority | Responsibility                                         |
| ------ | -------- | ------------------------------------------------------ |
| Health | 5        | 500 ms check-ins, watchdog feed, CONFIRMED record      |
| Comms  | 4        | HWLINK frames on USART3, commands and events            |
| Button | 3        | B1 EXTI wake-up, 30 ms debounce, short/long press       |
| LED    | 2        | LED patterns on LD1/LD2, LD3 latched for faults         |

Priorities are defined once in `src/lib.rs`; Rust tasks communicate through
`src/app/ipc.rs`, which uses fixed-size storage and critical sections only.

## Build pipeline

```
Cargo (Rust staticlib)  ─┐
C platform + HAL        ─┼─ CMake + Ninja ─ ELF ─ objcopy ─ .bin/.hex
FreeRTOS + CMSIS        ─┘                       └─ mkimage.py ─ .img
```

`scripts/build.sh` (or `make`) runs the whole pipeline inside the
`stm32-dev-rust-freertos` container. The post-build step also checks the
128 KB flash / 64 KB RAM budget with `scripts/check_footprint.py`. See the
top-level `README.md` for the user-facing build and flash targets.
