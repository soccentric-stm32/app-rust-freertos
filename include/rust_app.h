/*
 * rust_app.h - entry points exported by the Rust staticlib
 *
 * Author:  Sandesh Ghimire <sandesh@soccentric.com>
 * SRS:     RFCON-003, RFRT-001..RFRT-005
 *
 * src/lib.rs declares the same functions with #[no_mangle] extern "C".
 * rust_startup() runs in the C main thread before the scheduler starts;
 * the four task entries have FreeRTOS's TaskFunction_t signature and loop
 * forever.
 */

#ifndef RUST_APP_H
#define RUST_APP_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Start-up sequence: decode the boot record, log, check VTOR and publish
 * the initial status.  Called after HAL, clocks, UART, GPIO and IWDG init. */
void rust_startup(uint32_t reset_cause, uint8_t slot, uint8_t trial,
                  uint8_t trial_count);

/* FreeRTOS task entry points (void (*)(void *)).  Each owns one application
 * task and never returns. */
void rust_health_task(void *arg);
void rust_comms_task(void *arg);
void rust_button_task(void *arg);
void rust_led_task(void *arg);

#ifdef __cplusplus
}
#endif

#endif /* RUST_APP_H */
