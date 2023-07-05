use core::arch::x86_64::_rdtsc;
use std::{thread, time};
use log::info;

#[inline]
pub fn get_rdtsc() -> u64 {
    unsafe { _rdtsc() }
}

#[inline]
pub fn get_one_sec_rdtsc() -> f64 {
    let begin = get_rdtsc();
    thread::sleep(time::Duration::from_secs(1));
    let end = get_rdtsc();
    info!("One sec is equal to {} cycles.", end-begin);
    (end - begin) as f64
}

#[inline]
pub fn convert_rdtsc_to_ns(num: u64) -> f64 {
    let one_sec = get_one_sec_rdtsc();
    let sec: f64 = num as f64 / one_sec;
    sec * ((1000 * 1000 * 1000) as f64)
}