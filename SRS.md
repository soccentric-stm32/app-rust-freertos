# Software Requirements Specification — NUCLEO-F767ZI Rust-on-FreeRTOS Demo Application

| | |
|---|---|
| **Document** | SRS-RFREERTOS-F767 |
| **Version** | 0.1 (draft) |
| **Author** | Sandesh Ghimire <sandesh@soccentric.com> |
| **Date** | 2026-11-09 |
| **Status** | Draft for review |
| **Hardware** | NUCLEO-F767ZI, STM32F767ZIT6 rev Z (see [Hardware.md](https://github.com/soccentric-stm32/validation-firmware/blob/HEAD/Hardware.md)) |
| **Software base** | Rust 1.99.1 (`thumbv7em-none-eabihf`, `no_std` staticlib), FreeRTOS V10.2.0, STM32CubeF7 v1.17.4 HAL, CMake/Ninja |

---

## Contents

1. [Introduction](#1-introduction)
2. [Overall description](#2-overall-description)
3. [General requirements](#3-general-requirements)
4. [Bootloader contract](#4-bootloader-contract)
5. [C/Rust architecture and task requirements](#5-crust-architecture-and-task-requirements)
6. [Functional requirements](#6-functional-requirements)
7. [HWLINK](#7-hwlink)
8. [Fault handling, safety and logging](#8-fault-handling-safety-and-logging)
9. [Build and non-functional requirements](#9-build-and-non-functional-requirements)
10. [Verification matrix](#10-verification-matrix)
- [Appendix A — C platform API](#appendix-a--c-platform-api)
- [Appendix B — Open items](#appendix-b--open-items)

---

## 1. Introduction

### 1.1 Purpose

This document specifies **RFRT**, the demo application of SRS-DEMO-F767
written in Rust and running directly on the FreeRTOS kernel. It does the same
job as the C FreeRTOS demo: LED patterns, the user button, HWLINK
communication with the desktop GUI, watchdog supervision and self-confirmation
after a health check. It completes the comparison set: C and Rust, each on
FreeRTOS and on Zephyr.

### 1.2 Scope

RFRT implements the **behaviour** of the C FreeRTOS demo (SRS-DEMO-F767
§6–§9) and its bootloader contract (§4). The application logic is Rust; the
board start-up, STM32 drivers, FreeRTOS task creation and the bootloader
contract stay in C. The two sides are linked into one image by CMake.

Where this SRS says "as [DEMO] X", requirement X of SRS-DEMO-F767 applies
unchanged; only its implementation differs. RFRT is a demo and template: it
has no networking, storage or peripherals other than LEDs, button, UART,
IWDG, RTC backup registers and the CONFIRMED metadata record.

### 1.3 Definitions

| Term | Meaning |
|---|---|
| **RFRT** | The Rust-on-FreeRTOS application specified here |
| **[DEMO]** | The C FreeRTOS demo application, SRS-DEMO-F767 (functional baseline) |
| **[BOOT]** | The A/B bootloader, SRS-BOOT-F767 |
| **[LINK]** | HWLINK protocol, validation-gui SRS Appendix A |
| **Platform layer** | `src/platform.c`: the audited C functions declared in `src/platform.h` |
| **Rust side** | `src/lib.rs` and its modules, built by Cargo as `librust_freertos_app.a` |
| **HWLINK** | UART framing and FlatBuffers protocol ([LINK]) |
| **Pending / Confirmed** | Bootloader trial states ([BOOT] §7) |

### 1.4 Requirement conventions

**shall / should / may**, IDs `<AREA>-<NNN>`, methods **T** (test),
**D** (demonstration), **I** (inspection), **A** (analysis), as in [BOOT] §1.4.

### 1.5 References

| ID | Document |
|---|---|
| [DEMO] | [app-c-freertos/SRS.md](https://github.com/soccentric-stm32/app-c-freertos/blob/HEAD/SRS.md) — SRS-DEMO-F767 |
| [BOOT] | [bootloader/SRS.md](https://github.com/soccentric-stm32/bootloader/blob/HEAD/SRS.md) — SRS-BOOT-F767 |
| [GUI] | [validation-gui/SRS.md](https://github.com/soccentric-stm32/validation-gui/blob/HEAD/SRS.md) — SRS-GUI-F767 |
| [LINK] | [GUI] Appendix A — HWLINK protocol (normative) |
| [FRTOS] | FreeRTOS V10.2.0 reference manual |
| [RM0410] | ST, STM32F76xxx/F77xxx reference manual |
| [UM1974] | ST, STM32 Nucleo-144 boards user manual |

---

## 2. Overall description

### 2.1 Product perspective

```
 Host PC                                  NUCLEO-F767ZI
+-----------------+   ST-LINK VCP    +--------------------------------------+
| HWVAL GUI       |<================>| USART3 ─> Rust Comms task ──> Rust LED|
|  demo page:     |  HWLINK frames   |                 │          │         |
|  status, LEDs,  |                  |                 v          v         |
|  "update" ──────|──> bootloader    |            Rust Health   LD1 LD2 LD3 |
+-----------------+                  |                 │                    |
                                     |   B1 ──> Rust Button task ──> LED   |
                                     |   IWDG <── Rust Health task          |
                                     |   (running from slot A or B, BOOT)   |
                                     +--------------------------------------+
```

The C `main()` performs the board start-up, calls `rust_startup()` and creates
four FreeRTOS tasks whose entry points are Rust functions. All application
state and policy after that point live in Rust.

### 2.2 Constraints

| ID | Constraint |
|---|---|
| RFCON-001 | RFRT shall be built with one command (`scripts/build.sh`) inside the project Docker image `stm32-dev-rust-freertos`, which extends the shared `stm32-dev` image. CMake/Ninja builds the C sources; CMake invokes Cargo for the Rust staticlib. |
| RFCON-002 | The Rust toolchain shall be pinned in `rust-toolchain.toml` and in `docker/Dockerfile`; FreeRTOS and HAL shall be the unmodified STM32CubeF7 v1.17.4 versions. |
| RFCON-003 | The Rust side shall be `#![no_std]`, allocation-free, and exported as a `staticlib`. C `main()` shall call `rust_startup(reset_cause, slot, trial, trial_count)` before the scheduler starts and shall create the four Rust task entry points with `xTaskCreate()`. |
| RFCON-004 | The C/Rust boundary shall be the functions in `src/platform.h`; `src/plat.rs` shall declare exactly the same functions. No application policy shall live in `src/platform.c`. |
| RFCON-005 | RFRT shall run only under BOOT, linked at `0x0800 C000` ([BOOT] LAY-002). It is not intended to be flashed alone at `0x0800 0000`. |
| RFCON-006 | RFRT shall use USART3 (PD8/PD9) for HWLINK and USART6 (PG14) for debug output, and shall never reconfigure PA13/PA14 (SWD) or PH0/PH1 (HSE). |

### 2.3 Assumptions

As [DEMO] §2.4: the board is provisioned for BOOT, LD1/LD2/LD3 are on
PB0/PB7/PB14 (high = on), B1 is on PC13 (high when pressed), and the ST-LINK
VCP works in both directions.

---

## 3. General requirements

| ID | Requirement | Method |
|---|---|---|
| RFGEN-001 | The build shall produce `build/rust-freertos-app.elf`, `.bin`, `.hex`, `.map` and the bootloader image `build/rust-freertos-app.img` made by `scripts/mkimage.py` ([BOOT] BHOST-001). | I |
| RFGEN-002 | RFRT shall embed the version from `VERSION`, the git commit and the build date; the same values shall go into the image header. `SOURCE_DATE_EPOCH` shall make the build reproducible. | I, T |
| RFGEN-003 | RFRT shall run the core at 216 MHz from the 8 MHz HSE bypass clock through the PLL (M = 8, N = 432, P = 2, Q = 9), APB1 /4, APB2 /2, voltage scale 1, over-drive, 7 flash wait states, ART, prefetch, I-cache and D-cache on (as [DEMO] DGEN-003). | T |
| RFGEN-004 | HardFault, MemManage, BusFault and UsageFault shall save CFSR, HFSR, MMFAR, BFAR and the stacked PC/LR to a no-init RAM area, show the fault pattern on LD3 for about 2 s and reset. Rust panics and FreeRTOS asserts shall take the same path. The saved record shall be reported in `Status` after the reset. | T |
| RFGEN-005 | The reset cause shall be read from `RCC_CSR` at start-up, kept for `Status`, then cleared (as [DEMO] DGEN-005). | T |
| RFGEN-006 | The HAL time base shall use TIM6, not SysTick, which belongs to the FreeRTOS kernel. | I |
| RFGEN-007 | All source files shall carry the author header of the project. | I |

---

## 4. Bootloader contract

RFRT shall meet [BOOT] §11 APP-001…APP-007, as [DEMO] DBT-001…DBT-006:

| ID | Requirement | Method |
|---|---|---|
| RFBT-001 | `ld/app.ld` shall place the vector table at `0x0800 C000` with a 976 KB slot and shall fail the build if the image exceeds it. | I |
| RFBT-002 | The first instructions of C `main()` shall set `SCB->VTOR` to `0x0800 C000`, refresh the IWDG and start the fault boot id before any interrupt can use the table. | I, T |
| RFBT-003 | RFRT shall configure its clocks from the reset state BOOT leaves ([BOOT] HND-001). | T |
| RFBT-004 | C `main()` shall read the hand-over record from `RTC_BKP2R` with `app_handover_read()` and pass slot, trial and trial counter to `rust_startup()`. A missing record shall report slot `UNKNOWN` and skip confirmation. | T |
| RFBT-005 | `EnterBootloader` shall write the "stay in BOOT" magic to `RTC_BKP1R`, send `Ack`, wait until the UART has sent it, and reset ([BOOT] ENT-002). | T |
| RFBT-006 | The only flash write RFRT performs shall be the `CONFIRMED` record of the running slot. RFRT shall never write option bytes, bank 1 sectors 0–1, the other bank or any slot sector. | I |

---

## 5. C/Rust architecture and task requirements

### 5.1 Start-up

C `main()` shall, in this order: set VTOR, kick the IWDG early, start the fault
boot id, capture the reset cause, run `HAL_Init()`, configure clocks and
cache/MPU, enable the MemManage/BusFault/UsageFault exceptions, read the
hand-over record, start logging, configure GPIO, initialise the IWDG and
initialise USART3. It shall then call `rust_startup()` and create the four
tasks before `vTaskStartScheduler()`.

### 5.2 Tasks

| Task | Priority | Stack (words) | Job |
|---|---|---|---|
| **Health** | `tskIDLE_PRIORITY + 5` | 1024 | Supervises task check-ins, is the only IWDG feeder, runs the health check and writes `CONFIRMED` |
| **Comms** | `tskIDLE_PRIORITY + 4` | 1536 | Receives HWLINK frames, executes commands, sends replies and events |
| **Button** | `tskIDLE_PRIORITY + 3` | 512 | Debounces B1 and sends short/long-press commands to the LED task |
| **LED** | `tskIDLE_PRIORITY + 2` | 512 | Runs the current pattern on LD1/LD2/LD3 |

| ID | Requirement | Method |
|---|---|---|
| RFRT-001 | RFRT shall use the FreeRTOS ARM_CM7 GCC port with the native API, a 1 kHz tick from SysTick and `heap_4`, as [DEMO] DRT-001. | I |
| RFRT-002 | C `main()` shall have exactly the four tasks above (plus the FreeRTOS idle and timer tasks) and shall create them all before `vTaskStartScheduler()`. C `main()` shall not allocate memory after the scheduler starts. | I |
| RFRT-003 | The task entry points, priorities, stack depths and names shall be exactly those of §5.2. A failed `xTaskCreate()` shall be logged and shall take the fatal startup path. | I, T |
| RFRT-004 | Every task except Health shall check in with Health at least once per second. Health shall stop refreshing the IWDG when a task misses its check-in for 2 s, so the board resets. Health shall be the only task that feeds the IWDG. | T |
| RFRT-005 | `Status` shall report each task's stack high-water mark (`plat_stack_unused()`); before the scheduler starts the value shall be reported as zero. | T |
| RFRT-006 | Tasks shall communicate through FreeRTOS queues/task notifications or the Rust IPC layer; no task shall write another task's state directly. The platform layer shall not contain application policy. | I |
| RFRT-007 | `configASSERT`, stack-overflow checking (method 2) and the malloc-failed hook shall be enabled and shall record the reason in the no-init fault area and reset (as [DEMO] DRT-002). | T |

---

## 6. Functional requirements

| ID | Requirement | Method |
|---|---|---|
| RFN-001 | **LEDs:** as [DEMO] LED-001…LED-007, including the patterns of [DEMO] Appendix A and the reserved use of LD3. RFRT shall drive LD1/LD2/LD3 through `plat_led_set(0/1/2, on)`. | T, D |
| RFN-002 | **Button:** as [DEMO] BTN-001…BTN-004. The Rust Button task shall call `plat_button_bind()` once, then debounce B1 on PC13 in software with a 30 ms debounce. The C `EXTI15_10_IRQHandler` shall clear EXTI line 13 and notify the bound task. | T, D |
| RFN-003 | **Communication:** as [DEMO] COM-001…COM-007 (messages of [DEMO] §8.1), using USART3 through the C UART3 DMA driver and `plat_uart_*()`. | T |
| RFN-004 | **Health and confirmation:** as [DEMO] HLT-001…HLT-005, including a `fail-health` Cargo feature equivalent to `DEMO_FAIL_HEALTH`/`RF_FAIL_HEALTH`. The CONFIRMED record shall be read through `plat_confirmed_read()` and written through `plat_confirm_running_slot()`. | T |
| RFN-005 | `HelloReply` shall report role `DEMO`, firmware name `rust-freertos-app`, the version from `VERSION` and the Rust/FreeRTOS versions in its version text. | T |

The LED patterns, button actions, message set, health criteria, task
check-in timing and CONFIRMED record format are those of SRS-DEMO-F767
Appendices and §§6–9; RFRT shall not invent different behaviour.

---

## 7. HWLINK

| ID | Requirement | Method |
|---|---|---|
| RFLNK-001 | The FlatBuffers codec shall follow the shared schema `protocol/hwlink.fbs` and the HWLINK protocol ([LINK] A.5); it shall be hand-written or generated, but verified against the reference encoder. | I, T |
| RFLNK-002 | The codec shall work in `no_std` mode with fixed-size buffers; no heap allocator shall be used for RX or TX. | I, T |
| RFLNK-003 | COBS framing and CRC-32/ISO-HDLC shall follow [LINK] A.2 and be interoperable with BOOT and the GUI. The C side provides the CRC helper used for the CONFIRMED record. | T |
| RFLNK-004 | Every received FlatBuffer shall be verified before use; malformed requests shall be answered with `Nak MALFORMED`, unknown requests with `Nak UNSUPPORTED`, without state change. | T |
| RFLNK-005 | `SetBaud` shall allow 115200 and 921600 with a revert to 115200 if no valid frame arrives within 5 s at the new rate (as [DEMO] COM-001). | T |

---

## 8. Fault handling, safety and logging

| ID | Requirement | Method |
|---|---|---|
| RFSAFE-001 | RFRT shall never unlock or write option bytes. | I |
| RFSAFE-002 | The flash confirmation helper shall accept only the running slot's CONFIRMED record address; Rust shall not pass an address to it. | I, T |
| RFSAFE-003 | RFRT shall drive only PB0, PB7 and PB14, read PC13 and use PD8/PD9 (USART3) and PG14 (USART6 debug TX). | I |
| RFSAFE-004 | Rust shall use `panic = "abort"`, `lto = true`, `codegen-units = 1` and `opt-level = "s"` for release; no allocator shall be linked. A Rust `panic` shall call `plat_fatal()`, which records `APP_FAULT_ASSERT` and resets. | I |
| RFSAFE-005 | `plat_fault_get()` shall copy the decoded fault record to Rust but shall not expose the C magic word or boot id. | I |

### Debug output (USART6)

USART3 carries HWLINK only. Debug lines go to USART6 TX on PG14 (Arduino D1),
115200 8N1, in the format BOOT uses: `[<ms>] <TAG> <E|W|I|D> <text>`
(milliseconds since reset, right-aligned to 7 digits, CR LF), tag `R-FRT`.

| ID | Requirement | Method |
|---|---|---|
| RFDBG-001 | RFRT shall log start (version, commit, slot, trial state, reset cause), the last fault, the health verdict, button presses and HWLINK-requested resets through `plat_log()`/`app_log()`. | D |
| RFDBG-002 | `app_log()` shall never block: it formats into a stack buffer and copies the line into a 1 KB ring drained by the USART6 TXE interrupt, which makes no FreeRTOS call. It shall be safe from tasks, interrupts and before the scheduler starts, and shall not allocate. | I |
| RFDBG-003 | A line that does not fit in the ring shall be dropped and counted, never block the caller. | I |
| RFDBG-004 | Before a deliberate reset RFRT shall flush the ring by polling, bounded in time, so the last lines are not lost. | I |

---

## 9. Build and non-functional requirements

| ID | Requirement | Method |
|---|---|---|
| RFNFR-001 | **Footprint:** the image shall be under 128 KB of flash and 64 KB of RAM; the build shall print both and fail if either budget is exceeded. | I |
| RFNFR-002 | **Start-up:** the LED pattern shall be visible within 300 ms of hand-over. | T |
| RFNFR-003 | **Reliability:** RFRT should run 24 h with the GUI polling `GetStatus` every 100 ms without a reset or a missed reply. | T |
| RFNFR-004 | **Code quality:** C sources shall build with `-Wall -Wextra -Werror` and pass the clang-tidy target; Rust shall be formatted with rustfmt. | I |
| RFNFR-005 | **Template:** the README shall explain the project layout and how to build, flash and start a new application from RFRT. | I |
| RFNFR-006 | **Reproducibility:** `SOURCE_DATE_EPOCH` shall be passed through to CMake and Cargo so the same source produces the same build metadata and image header. | I |

---

## 10. Verification matrix

| Area | Requirements | Main method |
|---|---|---|
| General | RFGEN-001…007 | T, I |
| Bootloader contract | RFBT-001…006 | T, I |
| Architecture/tasks | RFCON-001…006, RFRT-001…007 | T, I |
| Functional | RFN-001…005 | T, D |
| HWLINK | RFLNK-001…005 | T, I |
| Safety/fault/log | RFSAFE-001…005, RFDBG-001…004 | I, T |
| Non-functional | RFNFR-001…006 | T, I, A |

---

## Appendix A — C platform API

`src/platform.h` is the complete boundary. `src/plat.rs` must declare the
same functions; the C code owns the magic words, boot id, flash address and
FreeRTOS handles.

| Group | Functions |
|---|---|
| GPIO/button | `plat_gpio_init`, `plat_led_set`, `plat_button_pressed`, `plat_button_wait`, `plat_button_bind` |
| USART3 | `plat_uart_init`, `plat_uart_set_baud`, `plat_uart_read`, `plat_uart_wait`, `plat_uart_rx_running`, `plat_uart_send`, `plat_uart_bind` |
| Watchdog | `plat_wdt_init`, `plat_wdt_feed` |
| Log/fault | `plat_log`, `plat_fault_boot_start`, `plat_fault_get`, `plat_fault_is_current_boot`, `plat_fault_is_previous_boot`, `plat_fault_clear`, `plat_fatal` |
| System/time | `plat_vtor`, `plat_reboot`, `plat_dcache_invalidate`, `plat_thread_name`, `plat_stack_unused`, `plat_reset_cause_flags`, `plat_handover_slot`, `plat_handover_trial`, `plat_handover_trial_count`, `plat_enter_bootloader`, `plat_now_ms`, `plat_sleep_ms` |
| Confirmation | `plat_confirmed_read`, `plat_confirm_running_slot` |

`uart3.c` calls `comms_post_rx_from_isr()`, which is implemented in
`platform.c` and posts to a one-slot RX queue consumed by
`plat_uart_wait()`.  The queue keeps the RX wake-up separate from the
TX-completion notification used by `app_uart3_send()`.

## Appendix B — Open items

| ID | Item | Status |
|---|---|---|
| OPEN-RF01 | Measure task stack high-water marks on hardware and reduce the 32 KB FreeRTOS heap if the Rust tasks need less. | Open bench item |
| OPEN-RF02 | Confirm the LD1 solder bridge (PB0 vs PA5) on this board (HWVAL OPEN-002). | Open bench item |
| OPEN-RF03 | Decide whether the C board/clock/fault files should move to a shared source directory with HWVAL ([DEMO] DGEN-007). | Open |
| OPEN-RF04 | Add a host-side test run for the Rust HWLINK codec with the shared protocol vectors. | Open |
