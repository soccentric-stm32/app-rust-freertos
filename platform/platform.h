/*
 * platform.h - C platform layer called from the Rust application
 *
 * Author:  Sandesh Ghimire <sandesh@soccentric.com>
 * SRS:     RFCON-004, RFBT-002, RFBT-005, RFRT-004, RFGEN-004
 *
 * The drivers and the bootloader contract stay in C (STM32 HAL and
 * FreeRTOS); the application logic is Rust.  src/platform/plat.rs declares the same
 * functions for Rust; keep the two files in step.  No application logic
 * lives here.
 */

#ifndef RFDEMO_PLATFORM_H
#define RFDEMO_PLATFORM_H

#include <stdbool.h>
#include <stdint.h>

#define PLAT_FAULT_TASK_LEN 16U
#define PLAT_FAULT_FILE_LEN 32U

/* Copy of the no-init fault record (same layout as the Rust Fault struct). */
typedef struct {
    uint32_t reason;
    uint32_t cfsr;
    uint32_t hfsr;
    uint32_t mmfar;
    uint32_t bfar;
    uint32_t pc;
    uint32_t lr;
    uint32_t psr;
    uint32_t line;
    char task[PLAT_FAULT_TASK_LEN];
    char file[PLAT_FAULT_FILE_LEN];
} plat_fault_t;

/* RFDBG-001: one debug line on USART6; level 'E', 'W', 'I' or 'D'.  The
 * text is copied (at most PLAT_LOG_TEXT_MAX bytes), not NUL-terminated. */
#define PLAT_LOG_TEXT_MAX 120U
void plat_log(uint8_t level, const char *text, uint32_t len);

/* Fault record (RFGEN-004). */
void plat_fault_boot_start(void);
bool plat_fault_get(plat_fault_t *out);
bool plat_fault_is_current_boot(void);
bool plat_fault_is_previous_boot(void);
void plat_fault_clear(void);
void plat_fatal(void) __attribute__((noreturn));

/* System. */
uint32_t plat_vtor(void);
void plat_reboot(void) __attribute__((noreturn));
void plat_dcache_invalidate(uintptr_t addr, uint32_t len);
void plat_thread_name(const char *name);
uint32_t plat_stack_unused(void);
uint32_t plat_reset_cause_flags(void);
uint8_t plat_handover_slot(void);
uint8_t plat_handover_trial(void);
uint8_t plat_handover_trial_count(void);
void plat_enter_bootloader(void) __attribute__((noreturn));

/* Time.  Milliseconds since boot and a task sleep. */
uint32_t plat_now_ms(void);
void plat_sleep_ms(uint32_t ms);

/* IWDG through the C driver (RFRT-004). */
bool plat_wdt_init(void);
void plat_wdt_feed(void);

/* LD1..LD3 and B1. */
bool plat_gpio_init(void);
void plat_led_set(uint32_t led, bool on);
bool plat_button_pressed(void);
void plat_button_wait(uint32_t timeout_ms);
void plat_button_bind(void);

/* USART3 HWLINK transport (RFN-003). */
bool plat_uart_init(void);
bool plat_uart_set_baud(uint32_t baud);
uint32_t plat_uart_read(uint8_t *dst, uint32_t max_len);
void plat_uart_wait(uint32_t timeout_ms);
bool plat_uart_rx_running(void);
bool plat_uart_send(const uint8_t *data, uint32_t len, uint32_t timeout_ms);
void plat_uart_bind(void);

/* The 32-byte CONFIRMED record in the running slot's metadata sector
 * (RFBT-006).  The offset is fixed here, never passed in. */
bool plat_confirmed_read(uint8_t *out32);
bool plat_confirm_running_slot(void);

#endif /* RFDEMO_PLATFORM_H */
