# Architecture: Rust on the FreeRTOS kernel

| | |
|---|---|
| **Author** | Sandesh Ghimire <sandesh@soccentric.com> |
| **Date** | 2026-11-09 |
| **Board** | NUCLEO-F767ZI |
| **SRS** | SRS-RFREERTOS-F767 |

## Why the C side exists

The Rust application is a `staticlib`, not a complete firmware. C keeps the
parts that must run before Rust can safely execute or that are tied to the
bootloader contract:

- the vector table and reset handler from STM32CubeF7;
- the 216 MHz clock, cache/MPU and HAL time base;
- the no-init fault record and the fatal path that works without HAL or Rust;
- the hand-over record and the single CONFIRMED flash write;
- FreeRTOS task creation, so the scheduler sees all four tasks at once;
- the small C platform API in `src/platform.h`.

Everything else — LED patterns, button debounce, HWLINK, health supervision,
status and fault reporting — is Rust.

## Start-up sequence

```
Reset_Handler
  -> C main()
       SCB->VTOR = APP_SLOT_BASE
       early IWDG kick, fault boot id
       capture reset cause (RCC_CSR)
       HAL_Init(), clock init, cache/MPU
       enable MemManage/BusFault/UsageFault
       read RTC_BKP2R hand-over record
       start USART6 debug log
       configure GPIO (LD1/LD2/LD3, B1 EXTI)
       start/refresh IWDG
       start USART3 DMA receiver
       rust_startup(reset_cause, slot, trial, trial_count)
       xTaskCreate(Health, Comms, Button, LED)
       vTaskStartScheduler()
```

`rust_startup()` runs in the C `main()` thread before the scheduler. It may
use HAL time, logging, GPIO and the watchdog through the platform layer.

## Task model

| Task | Priority | Stack | C wake-up source | Rust work |
|---|---|---|---|---|
| Health | idle + 5 | 1024 words | 100 ms sleep | check-ins, health verdict, CONFIRMED, IWDG |
| Comms | idle + 4 | 1536 words | USART3 RX queue, 20 ms timeout | HWLINK RX/TX, commands, events |
| Button | idle + 3 | 512 words | EXTI15_10 notification, 10 ms timeout | debounce, short/long press |
| LED | idle + 2 | 512 words | 1 ms sleep | LED pattern state machine |

All tasks are created before `vTaskStartScheduler()`. The FreeRTOS heap is
32 KB by default because Rust formatting and the HWLINK codec use more stack
than the equivalent C code; `APP_HEAP_KB` can be changed at configure time.

## C/Rust boundary

`src/platform.h` / `src/plat.rs` are the complete boundary. The C side owns
all addresses, magic words, boot ids and FreeRTOS handles. Rust does not see
the fault magic, the boot id or a caller-supplied flash address.

- GPIO and button: direct register access for the three LEDs; PC13 EXTI wakes
  the bound task through a task notification.
- USART3: the existing DMA ring driver. `comms_post_rx_from_isr()` is
  implemented in `platform.c` and posts to a one-slot RX queue, so the RX
  wake-up cannot be confused with the TX-completion notification used by
  `app_uart3_send()`.
- IWDG: `app_iwdg_init()` / `app_iwdg_kick()`.
- Fault: `plat_fault_get()` copies a decoded `plat_fault_t`; `plat_fatal()`
  records a Rust panic through `app_fault_fatal()`.
- CONFIRMED: `plat_confirmed_read()` invalidates the D-cache and copies the
  32-byte record; `plat_confirm_running_slot()` calls the audited C helper.

## Build and link

CMake compiles the C objects and FreeRTOS kernel first. A custom command runs
`cargo build --release` with `CARGO_TARGET_DIR` under `build/rust` and
`CARGO_HOME` in the project. The resulting
`librust_freertos_app.a` is linked after the C objects (and libgcc) so that
symbols referenced by `main.c` pull the Rust archive members in. The linker
script keeps the vector table at `0x0800C000`, the fault record in the
reserved no-init window and DMA buffers in SRAM2.
