// boot.rs - bootloader hand-over, reset cause and CONFIRMED record
//
// Author: Sandesh Ghimire <sandesh@soccentric.com>
// SRS:    RFBT-004..006, RFGEN-005, RFN-004, RFSAFE-002
//
// BOOT and the C platform layer own the backup-domain access and the flash
// programming.  Rust only decodes the records and drives the health check.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::platform::board;
use crate::platform::plat;
use crate::protocol::hwlink::frame::crc32;

#[derive(Clone, Copy)]
pub struct Handover {
    pub slot: u8,
    pub trial_state: u8,
    pub trial_count: u8,
}

impl Handover {
    pub const fn unknown() -> Self {
        Self {
            slot: board::SLOT_UNKNOWN,
            trial_state: board::TRIAL_UNKNOWN,
            trial_count: 0,
        }
    }

    pub fn pending(&self) -> bool {
        self.trial_state == board::TRIAL_PENDING
    }

    /// Trial state to report now.  The hand-over is decoded once at start-up,
    /// so a slot that was on trial reports CONFIRMED once the health task
    /// has written the CONFIRMED record.
    pub fn current_trial_state(&self) -> u8 {
        if self.pending() && CONFIRMED.load(Ordering::Acquire) {
            board::TRIAL_CONFIRMED
        } else {
            self.trial_state
        }
    }
}

/// Set once the CONFIRMED record for the running slot is in flash.
static CONFIRMED: AtomicBool = AtomicBool::new(false);

/// Record that the running slot is no longer on trial.
pub fn mark_confirmed() {
    CONFIRMED.store(true, Ordering::Release);
}

/// RFGEN-005: the reset flags were captured and cleared by C before Rust
/// started; return that saved copy for `Status`.
#[allow(dead_code)]
pub fn take_reset_cause() -> u32 {
    plat::reset_cause_flags()
}

pub fn reset_was_watchdog(cause: u32) -> bool {
    cause & (board::RCC_CSR_WDGRSTF | board::RCC_CSR_WWDGRSTF) != 0
}

/// Text used by `Status`; the order matches the Rust Embassy demo.
pub fn reset_cause_text(cause: u32) -> &'static str {
    if cause & board::RCC_CSR_WDGRSTF != 0 {
        "independent watchdog"
    } else if cause & board::RCC_CSR_WWDGRSTF != 0 {
        "window watchdog"
    } else if cause & board::RCC_CSR_SFTRSTF != 0 {
        "software"
    } else if cause & board::RCC_CSR_LPWRRSTF != 0 {
        "low power"
    } else if cause & board::RCC_CSR_BORRSTF != 0 {
        "brown-out"
    } else if cause & board::RCC_CSR_PINRSTF != 0 {
        "reset pin"
    } else if cause & board::RCC_CSR_PORRSTF != 0 {
        "power-on"
    } else {
        "unknown"
    }
}

/// RFBT-004: read the hand-over record that C decoded from RTC_BKP2R.
#[allow(dead_code)]
pub fn read_handover() -> Handover {
    let slot = plat::handover_slot();
    if slot != board::SLOT_A && slot != board::SLOT_B {
        return Handover::unknown();
    }
    let trial_state = match plat::handover_trial() {
        board::TRIAL_CONFIRMED => board::TRIAL_CONFIRMED,
        board::TRIAL_PENDING => board::TRIAL_PENDING,
        _ => board::TRIAL_UNKNOWN,
    };
    Handover {
        slot,
        trial_state,
        trial_count: plat::handover_trial_count(),
    }
}

/// RFBT-005: leave a request for BOOT and reset.
pub fn enter_bootloader() -> ! {
    log_info!("HWLINK: EnterBootloader, resetting into BOOT");
    plat::enter_bootloader()
}

pub fn reset() -> ! {
    log_info!("HWLINK: Reset");
    plat::reboot()
}

/// RFBT-006 / RFSAFE-002: the only flash write in RFDEMO.  The record
/// address is fixed by platform.h; nothing here can name another one.
pub fn write_confirmed() -> bool {
    match plat::confirmed_read() {
        Some(current) => {
            if confirmed_valid(&current) {
                // HLT-005: an already confirmed slot does not write flash again.
                return true;
            }
            if current.iter().any(|&b| b != 0xFF) {
                // Append-only: a torn or foreign record is never overwritten.
                return false;
            }
        }
        None => {}
    }

    // C builds and programs the same 32-byte record as BOOT expects.
    if !plat::confirm_running_slot() {
        return false;
    }

    // Read the record back from flash, not from a program buffer.
    plat::confirmed_read().is_some_and(|verify| confirmed_valid(&verify))
}

fn confirmed_valid(record: &[u8; 32]) -> bool {
    let magic = u32::from_le_bytes([record[0], record[1], record[2], record[3]]);
    if magic != board::CONFIRMED_MAGIC {
        return false;
    }
    let stored = u32::from_le_bytes([record[28], record[29], record[30], record[31]]);
    crc32(&record[..28]) == stored
}

pub fn device_id() -> (u32, u32) {
    // SAFETY: read-only DBGMCU identity register.
    let id = unsafe { board::DBGMCU_IDCODE.read_volatile() };
    let dev = id & 0x0FFF;
    let rev = (id >> 16) & 0xFFFF;
    (if dev == 0 { board::DBGMCU_DEV_ID } else { dev }, rev)
}

/// Same 64-bit value as the C demo: UID word 0 in the high half, word 1 low.
pub fn device_uid() -> u64 {
    // SAFETY: read-only factory UID words in system memory.
    unsafe {
        let w0 = board::UID_BASE.read_volatile() as u64;
        let w1 = board::UID_BASE.add(1).read_volatile() as u64;
        (w0 << 32) | w1
    }
}
