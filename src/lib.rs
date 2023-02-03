// \file lib.rs
// \copyright Copyright (C) Infineon Technologies AG 2023
// 
// Use of this file is subject to the terms of use agreed between (i) you or the company in which ordinary course of
// business you are acting and (ii) Infineon Technologies AG or its licensees. If and as long as no such terms of use
// are agreed, use of this file is subject to following:
// 
// Boost Software License - Version 1.0 - August 17th, 2003
// 
// Permission is hereby granted, free of charge, to any person or organization obtaining a copy of the software and
// accompanying documentation covered by this license (the "Software") to use, reproduce, display, distribute, execute,
// and transmit the Software, and to prepare derivative works of the Software, and to permit third-parties to whom the
// Software is furnished to do so, all subject to the following:
// 
// The copyright notices in the Software and this entire statement, including the above license grant, this restriction
// and the following disclaimer, must be included in all copies of the Software, in whole or in part, and all
// derivative works of the Software, unless such copies or derivative works are solely in the form of
// machine-executable object code generated by a source language processor.
// 
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE
// WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE, TITLE AND NON-INFRINGEMENT. IN NO EVENT SHALL THE
// COPYRIGHT HOLDERS OR ANYONE DISTRIBUTING THE SOFTWARE BE LIABLE FOR ANY DAMAGES OR OTHER LIABILITY, WHETHER IN
// CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
// IN THE SOFTWARE.
// ---

#![no_std]
#![no_main]

use panic_halt as _;

use cyt2b7 as pac;
use pac::CPUSS;
use pac::SRSS;
use pac::FLASHC;

#[cfg(all(cm4))]
pub mod cortex_m4;

#[cfg(all(cm0))]
pub mod cortex_m0;

#[cfg(all(cm4))]
const CORE_FREQUENCY: u32 = 160_000_000;

#[cfg(all(cm0))]
const CORE_FREQUENCY: u32 = 80_000_000;

unsafe fn setup_memory_wait_states() {
    // ROM wait states...
    (*CPUSS::ptr()).rom_ctl.write(|w| w.slow_ws().bits(1));
    (*CPUSS::ptr()).rom_ctl.write(|w| w.fast_ws().bits(0));
    
    // RAM 0 wait states...
    (*CPUSS::ptr()).ram0_ctl0.write(|w| w.slow_ws().bits(1));
    (*CPUSS::ptr()).ram0_ctl0.write(|w| w.fast_ws().bits(0));

    // RAM 1 wait states...
    (*CPUSS::ptr()).ram1_ctl0.write(|w| w.slow_ws().bits(1));
    (*CPUSS::ptr()).ram1_ctl0.write(|w| w.fast_ws().bits(0));

    // Flash wait states...
    (*FLASHC::ptr()).flash_ctl.write(|w| w.main_ws().bits(1));
}

/// Unlock the watchdog timer (WDT)
pub unsafe fn unlock_wdt() {
    (*SRSS::ptr()).wdt.lock.write(|w| w.wdt_lock().bits(1));
    (*SRSS::ptr()).wdt.lock.write(|w| w.wdt_lock().bits(2));
}

/// Lock the watchdog timer (WDT)
pub unsafe fn lock_wdt() {
    (*SRSS::ptr()).wdt.lock.write(|w| w.wdt_lock().bits(3));
}

/// Return core freqeuncy for the current core
pub fn get_core_frequency() -> u32 {
    CORE_FREQUENCY
}

/// Initialize all clocks using IMO as the System clock 
pub unsafe fn clock_init() {
    // Disable the watchdog...
    unlock_wdt();
    (*SRSS::ptr()).wdt.ctl.write(|w| w.enable().bit(false));
    lock_wdt();

    setup_memory_wait_states();

    // Set LF clock source...
    (*SRSS::ptr()).clk_select.write(|w| w.lfclk_sel().bits(0));

    // Set CPUSS dividers...
    // - FAST (CM4) = 160,000,000
    // - PERI (CM0) = Divided by 2
    // - SLOW (CM0) == PERI (CM0)
    (*CPUSS::ptr()).cm4_clock_ctl.write(|w| w.fast_int_div().bits(0));
    (*CPUSS::ptr()).cm0_clock_ctl.write(|w| w.bits(0x01000000));

    // Set and enable PLL0...
    // - FEEDBACK_DIV = 1
    // - REFERENCE_DIV = 40
    // - OUTPUT_DIV = 2
    // - ENABLE = 1
    (*SRSS::ptr()).clk_path_select[1].write(|w| w.path_mux().bits(0));
    (*SRSS::ptr()).clk_pll_config[0].write(|w| w.bits(0x80020128));

    // Wait for a PLL lock...
    while (*SRSS::ptr()).clk_pll_status[0].read().locked().bit_is_clear() {}

    // Set path 2 source
    (*SRSS::ptr()).clk_path_select[2].write(|w| w.path_mux().bits(0));

    // Enable HF0 clock with PLL0 as source and ROOT_DIV == NO_DIV...
    (*SRSS::ptr()).clk_root_select[0].write(|w| w.bits(0x80000001));

    // Enable HF1 clock with PLL0 as source and ROOT_DIV == DIV_BY_2...
    (*SRSS::ptr()).clk_root_select[1].write(|w| w.bits(0x80000011));

    // Enable ILO0...
    unlock_wdt();
    (*SRSS::ptr()).clk_ilo0_config.write(|w| w.enable().bit(true));
    (*SRSS::ptr()).clk_ilo0_config.write(|w| w.ilo0_backup().bit(true));
    lock_wdt();
}

/// Initialize the vector table for the CM4 core in the CPUSS_CM4_VECTOR_TABLE_BASE
/// register with the start address of the vector table and then enable power to
/// the CM4 core
pub unsafe fn enable_cm4() {
    // Set the CM4 vector table to the start of address of the vector table,
    // which is at the beginning of the FLASH assigned to the CM4 core (see
    // the memory_cm4.x linker file). This has to be done before starting the
    // CM4 core
    (*CPUSS::ptr())
        .cm4_vector_table_base
        .write(|w| w.bits(0x10008000));

    // Start the CM4 core
    (*CPUSS::ptr()).cm4_pwr_ctl.write(|w| w.bits(0x05fa0003) );
}
