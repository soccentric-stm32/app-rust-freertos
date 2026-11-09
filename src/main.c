/*
 * main.c - Rust-on-FreeRTOS application entry point
 *
 * Author:  Sandesh Ghimire <sandesh@soccentric.com>
 * Date:    2026-11-09
 * Target:  NUCLEO-F767ZI (STM32F767ZIT6), started by BOOT from slot A or B
 * SRS:     SRS-RFREERTOS-F767 (RFCON-001, RFBT-001..RFBT-006, RFRT-001..005)
 *
 * The C side keeps the proven board start-up: vector table, watchdog, fault
 * record, clocks, caches/MPU, hand-over, logging, GPIO, IWDG and USART3.
 * The application itself is Rust: rust_startup() runs here in the C main
 * thread, then the four Rust task entry points become FreeRTOS tasks.  All
 * tasks are created before vTaskStartScheduler(); nothing allocates after
 * the scheduler starts.
 */

#include "stm32f7xx_hal.h"
#include "FreeRTOS.h"
#include "task.h"

#include "app_fault.h"
#include "app_log.h"
#include "boot_contract.h"
#include "cache_mpu.h"
#include "clock.h"
#include "handover.h"
#include "iwdg.h"
#include "platform.h"
#include "reset_cause.h"
#include "rust_app.h"

/* Implemented in src/platform.c; also exposed to Rust as plat_gpio_init(). */
void app_gpio_init(void);

static void startup_fail(const char *where) __attribute__((noreturn));
static void startup_fail(const char *where)
{
    LOG_E("start-up failed: %s", where);
    app_log_flush(50U);
    app_fault_fatal(APP_FAULT_STARTUP, "main", where, 0U);
}

int main(void)
{
    /* RFBT-002: first instructions, before any interrupt can use the table. */
    SCB->VTOR = APP_SLOT_BASE;
    app_iwdg_early_kick();
    app_fault_boot_start();

    /* RFGEN-005: keep the flags for Status, then clear them. */
    app_reset_cause_capture();

    (void)HAL_Init();
    app_clock_init();
    app_cache_mpu_init();

    /* RFGEN-004: take MemManage/BusFault/UsageFault to their own handlers so
     * the saved reason is specific; HardFault catches escalations. */
    SCB->SHCSR |= SCB_SHCSR_MEMFAULTENA_Msk | SCB_SHCSR_BUSFAULTENA_Msk |
                  SCB_SHCSR_USGFAULTENA_Msk;
    __DSB();
    __ISB();

    /* RFBT-004 / RFBT-005: hand-over record and backup-domain access. */
    app_handover_read();

    /* RFDBG-001: debug lines on USART6 from here on. */
    app_log_init();
    LOG_I("start v" APP_VERSION_STR " " APP_GIT_COMMIT ", slot %c %s, "
          "reset %s",
          g_handover.slot == APP_SLOT_A ? 'A' :
          g_handover.slot == APP_SLOT_B ? 'B' : '?',
          g_handover.trial == 2U ? "on trial" :
          g_handover.trial == 1U ? "confirmed" : "no hand-over",
          app_reset_cause_text);

    /* Board I/O and the watchdog service are ready before Rust starts. */
    app_gpio_init();
    app_iwdg_init();

    /* RFN-003: USART3 must be running before Rust's health check. */
    if (!plat_uart_init()) {
        startup_fail("uart3 init");
    }

    /* RFCON-003: hand the reset cause and boot record to Rust.  This runs in
     * the C main thread before the scheduler starts. */
    rust_startup(app_reset_cause_flags, g_handover.slot, g_handover.trial,
                 g_handover.trial_count);

    /* RFRT-001..RFRT-005: exactly four application tasks.  Priorities and
     * stack depths in words are fixed by the SRS; Rust formatting needs
     * more stack than the equivalent C code. */
    if (xTaskCreate(rust_health_task, "Health", 1024U, NULL,
                    tskIDLE_PRIORITY + 5U, NULL) != pdPASS) {
        startup_fail("health task");
    }
    if (xTaskCreate(rust_comms_task, "Comms", 1536U, NULL,
                    tskIDLE_PRIORITY + 4U, NULL) != pdPASS) {
        startup_fail("comms task");
    }
    if (xTaskCreate(rust_button_task, "Button", 512U, NULL,
                    tskIDLE_PRIORITY + 3U, NULL) != pdPASS) {
        startup_fail("button task");
    }
    if (xTaskCreate(rust_led_task, "LED", 512U, NULL,
                    tskIDLE_PRIORITY + 2U, NULL) != pdPASS) {
        startup_fail("led task");
    }

    vTaskStartScheduler();

    /* Only reached when the idle or timer task could not be created. */
    startup_fail("scheduler");
}
