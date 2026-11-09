// plat.rs - Rust side of the C platform layer (src/platform.h)
//
// Author: Sandesh Ghimire <sandesh@soccentric.com>
// SRS:    RFRT-004, RFCON-004
//
// Declarations must match src/platform.h.  Every wrapper here is safe: the C
// functions take fixed arguments and check their own bounds.  The FreeRTOS
// kernel and the STM32 drivers stay in C.  Some entries are only part of the
// platform contract and are not called by the current Rust application.

#![allow(dead_code)]

use core::ffi::{c_char, CStr};

pub const FAULT_TASK_LEN: usize = 16;
pub const FAULT_FILE_LEN: usize = 32;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Fault {
    pub reason: u32,
    pub cfsr: u32,
    pub hfsr: u32,
    pub mmfar: u32,
    pub bfar: u32,
    pub pc: u32,
    pub lr: u32,
    pub psr: u32,
    pub line: u32,
    pub task: [u8; FAULT_TASK_LEN],
    pub file: [u8; FAULT_FILE_LEN],
}

impl Fault {
    const fn empty() -> Self {
        Self {
            reason: 0,
            cfsr: 0,
            hfsr: 0,
            mmfar: 0,
            bfar: 0,
            pc: 0,
            lr: 0,
            psr: 0,
            line: 0,
            task: [0; FAULT_TASK_LEN],
            file: [0; FAULT_FILE_LEN],
        }
    }
}

mod ffi {
    use core::ffi::c_char;

    use super::Fault;

    extern "C" {
        pub fn plat_log(level: u8, text: *const u8, len: u32);

        pub fn plat_fault_boot_start();
        pub fn plat_fault_get(out: *mut Fault) -> bool;
        pub fn plat_fault_is_current_boot() -> bool;
        pub fn plat_fault_is_previous_boot() -> bool;
        pub fn plat_fault_clear();
        pub fn plat_fatal() -> !;

        pub fn plat_vtor() -> u32;
        pub fn plat_reboot() -> !;
        pub fn plat_dcache_invalidate(addr: usize, len: u32);
        pub fn plat_thread_name(name: *const c_char);
        pub fn plat_stack_unused() -> u32;
        pub fn plat_reset_cause_flags() -> u32;
        pub fn plat_handover_slot() -> u8;
        pub fn plat_handover_trial() -> u8;
        pub fn plat_handover_trial_count() -> u8;
        pub fn plat_enter_bootloader() -> !;

        pub fn plat_now_ms() -> u32;
        pub fn plat_sleep_ms(ms: u32);

        pub fn plat_wdt_init() -> bool;
        pub fn plat_wdt_feed();

        pub fn plat_gpio_init() -> bool;
        pub fn plat_led_set(led: u32, on: bool);
        pub fn plat_button_pressed() -> bool;
        pub fn plat_button_wait(timeout_ms: u32);
        pub fn plat_button_bind();

        pub fn plat_uart_init() -> bool;
        pub fn plat_uart_set_baud(baud: u32) -> bool;
        pub fn plat_uart_read(dst: *mut u8, max_len: u32) -> u32;
        pub fn plat_uart_wait(timeout_ms: u32);
        pub fn plat_uart_rx_running() -> bool;
        pub fn plat_uart_send(data: *const u8, len: u32, timeout_ms: u32) -> bool;
        pub fn plat_uart_bind();

        pub fn plat_confirmed_read(out32: *mut u8) -> bool;
        pub fn plat_confirm_running_slot() -> bool;
    }
}

// SAFETY (all wrappers below): the C functions take no pointers, or only
// pointers to buffers of the length passed with them, and are safe to call
// once the FreeRTOS kernel runs.

pub fn fault_boot_start() {
    unsafe { ffi::plat_fault_boot_start() }
}

pub fn fault_get() -> Option<Fault> {
    let mut fault = Fault::empty();
    if unsafe { ffi::plat_fault_get(&mut fault) } {
        Some(fault)
    } else {
        None
    }
}

pub fn fault_is_current_boot() -> bool {
    unsafe { ffi::plat_fault_is_current_boot() }
}

pub fn fault_is_previous_boot() -> bool {
    unsafe { ffi::plat_fault_is_previous_boot() }
}

pub fn fault_clear() {
    unsafe { ffi::plat_fault_clear() }
}

pub fn fatal() -> ! {
    unsafe { ffi::plat_fatal() }
}

pub fn vtor() -> u32 {
    unsafe { ffi::plat_vtor() }
}

pub fn reboot() -> ! {
    unsafe { ffi::plat_reboot() }
}

/// RFRT-001: one debug line on USART6 (see src/log.rs for the macros).
pub fn log(level: u8, text: &[u8]) {
    unsafe { ffi::plat_log(level, text.as_ptr(), text.len() as u32) }
}

pub fn dcache_invalidate(addr: u32, len: u32) {
    unsafe { ffi::plat_dcache_invalidate(addr as usize, len) }
}

pub fn thread_name(name: &'static CStr) {
    unsafe { ffi::plat_thread_name(name.as_ptr() as *const c_char) }
}

/// Unused bytes on the calling task's stack (RFRT-005).
pub fn stack_unused() -> u32 {
    unsafe { ffi::plat_stack_unused() }
}

/// Reset flags captured and cleared by C before Rust starts.
pub fn reset_cause_flags() -> u32 {
    unsafe { ffi::plat_reset_cause_flags() }
}

pub fn handover_slot() -> u8 {
    unsafe { ffi::plat_handover_slot() }
}

pub fn handover_trial() -> u8 {
    unsafe { ffi::plat_handover_trial() }
}

pub fn handover_trial_count() -> u8 {
    unsafe { ffi::plat_handover_trial_count() }
}

/// Ask BOOT to stay in the bootloader, then reset.
pub fn enter_bootloader() -> ! {
    unsafe { ffi::plat_enter_bootloader() }
}

pub fn wdt_init() -> bool {
    unsafe { ffi::plat_wdt_init() }
}

pub fn wdt_feed() {
    unsafe { ffi::plat_wdt_feed() }
}

pub fn gpio_init() -> bool {
    unsafe { ffi::plat_gpio_init() }
}

/// `led` is 0 for LD1, 1 for LD2 and 2 for LD3.
pub fn led_set(led: u32, on: bool) {
    unsafe { ffi::plat_led_set(led, on) }
}

pub fn button_pressed() -> bool {
    unsafe { ffi::plat_button_pressed() }
}

/// Wait for a B1 edge or the timeout, whichever comes first.
pub fn button_wait(timeout_ms: u32) {
    unsafe { ffi::plat_button_wait(timeout_ms) }
}

/// Let the C EXTI handler notify the calling task on a B1 edge.
pub fn button_bind() {
    unsafe { ffi::plat_button_bind() }
}

pub fn uart_init() -> bool {
    unsafe { ffi::plat_uart_init() }
}

pub fn uart_set_baud(baud: u32) -> bool {
    unsafe { ffi::plat_uart_set_baud(baud) }
}

pub fn uart_read(dst: &mut [u8]) -> usize {
    unsafe { ffi::plat_uart_read(dst.as_mut_ptr(), dst.len() as u32) as usize }
}

/// Wait for received bytes or the timeout, whichever comes first.
pub fn uart_wait(timeout_ms: u32) {
    unsafe { ffi::plat_uart_wait(timeout_ms) }
}

pub fn uart_rx_running() -> bool {
    unsafe { ffi::plat_uart_rx_running() }
}

/// Send one frame and wait until its last stop bit has left the UART.
pub fn uart_send(data: &[u8], timeout_ms: u32) -> bool {
    unsafe { ffi::plat_uart_send(data.as_ptr(), data.len() as u32, timeout_ms) }
}

/// Kept for the platform contract; USART3 RX wakes through a C queue.
pub fn uart_bind() {
    unsafe { ffi::plat_uart_bind() }
}

pub fn confirmed_read() -> Option<[u8; 32]> {
    let mut record = [0u8; 32];
    if unsafe { ffi::plat_confirmed_read(record.as_mut_ptr()) } {
        Some(record)
    } else {
        None
    }
}

/// Write the CONFIRMED record for the running slot through C.
pub fn confirm_running_slot() -> bool {
    unsafe { ffi::plat_confirm_running_slot() }
}

/// Milliseconds since boot.
pub fn now_ms() -> u64 {
    unsafe { ffi::plat_now_ms() as u64 }
}

pub fn sleep_ms(ms: u32) {
    unsafe { ffi::plat_sleep_ms(ms) }
}
