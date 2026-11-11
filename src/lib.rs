// rust-freertos-app - Rust demo application on FreeRTOS, entry point
//
// Author:  Sandesh Ghimire <sandesh@soccentric.com>
// Date:    2026-11-04
// Target:  NUCLEO-F767ZI (STM32F767ZIT6), started by BOOT from slot A or B
// SRS:     SRS-RFREERTOS-F767, SRS-DEMO-F767
//
// C main() initialises the HAL, clocks, GPIO, USART6, USART3 and IWDG, then
// calls rust_startup() before the scheduler starts.  It creates the four
// application tasks afterwards from the entries exported by health.rs,
// comms.rs, button.rs and led.rs.  The only flash write is the CONFIRMED
// record in boot.rs.

#![no_std]
#![deny(warnings)]

use core::ffi::c_void;

#[macro_use]
mod log;

mod app;
mod platform;
mod protocol;

// RFRT-001: Health highest, Comms high, Button and LED normal.  These match
// the C FreeRTOS demo's task priorities (the stack sizes live in C).
pub(crate) const PRIO_HEALTH: i32 = 5;
pub(crate) const PRIO_COMMS: i32 = 4;
pub(crate) const PRIO_BUTTON: i32 = 3;
pub(crate) const PRIO_LED: i32 = 2;

// Compile-time check that every entry matches FreeRTOS's TaskFunction_t
// (void (*)(void *)); the functions themselves are exported by their modules.
const _: [extern "C" fn(*mut c_void); 4] = [
    app::health::rust_health_task,
    app::comms::rust_comms_task,
    app::button::rust_button_task,
    app::led::rust_led_task,
];

#[no_mangle]
pub extern "C" fn rust_startup(reset_cause: u32, slot: u8, trial: u8, trial_count: u8) {
    // RFRT-004: C decoded the hand-over record and captured the reset flags
    // before calling us; keep the same values for the tasks.
    let handover = if slot == platform::board::SLOT_A || slot == platform::board::SLOT_B {
        platform::boot::Handover {
            slot,
            trial_state: trial,
            trial_count,
        }
    } else {
        platform::boot::Handover::unknown()
    };
    app::ipc::set_boot_info(app::ipc::BootInfo {
        reset_cause,
        handover,
    });

    // RFDBG-001: start-up line on the debug UART.
    log_info!(
        "start v{} {}, slot {} {}, reset {}",
        platform::build_info::VERSION,
        platform::build_info::GIT_COMMIT,
        match handover.slot {
            platform::board::SLOT_A => 'A',
            platform::board::SLOT_B => 'B',
            _ => '?',
        },
        match handover.trial_state {
            platform::board::TRIAL_PENDING => "on trial",
            platform::board::TRIAL_CONFIRMED => "confirmed",
            _ => "no hand-over",
        },
        platform::boot::reset_cause_text(reset_cause)
    );

    // RFBT-001/002: the vector table must be the image's own table.
    if platform::plat::vtor() != platform::board::SLOT_BASE {
        app::fault::fatal();
    }

    // LED-005: latch LD3 for the last fault or watchdog reset.
    let last_fault = app::fault::last();
    let initial_fault = last_fault.is_some() || platform::boot::reset_was_watchdog(reset_cause);
    if last_fault.is_some() {
        log_warn!("last fault recorded before this reset");
    }
    app::ipc::update_status(|s| s.fault_active = initial_fault);
}

/// RFDBG-001/RFSAFE-003: a Rust panic becomes one debug line and the C fatal
/// path.  The buffer is fixed-size; formatting never allocates.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::{self, Write as _};

    struct Buffer {
        data: [u8; 120],
        len: usize,
    }

    impl fmt::Write for Buffer {
        fn write_str(&mut self, text: &str) -> fmt::Result {
            let take = text.len().min(self.data.len() - self.len);
            self.data[self.len..self.len + take].copy_from_slice(&text.as_bytes()[..take]);
            self.len += take;
            Ok(())
        }
    }

    let mut line = Buffer {
        data: [0; 120],
        len: 0,
    };
    let _ = line.write_fmt(format_args!("panic: {}", info.message()));
    platform::plat::log(b'E', &line.data[..line.len]);
    platform::plat::fatal();
}
