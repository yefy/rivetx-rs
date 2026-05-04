#![allow(deprecated)]

use log::error;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref THREAD_PANIC_MUTEX: Mutex<()> = Mutex::new(());
}

pub fn thread_panic(panic_info: &std::panic::PanicInfo) {
    let _unused = THREAD_PANIC_MUTEX.lock().unwrap();

    let curr_thread = std::thread::current();
    let curr_thread_name = curr_thread.name().unwrap_or("thread_panic");

    let payload = match panic_info.payload().downcast_ref::<&'static str>() {
        Some(payload) => *payload,
        None => match panic_info.payload().downcast_ref::<String>() {
            Some(payload) => &payload[..],
            None => "payload",
        },
    };

    let location = if let Some(location) = panic_info.location() {
        location.to_string()
    } else {
        "location".to_string()
    };

    println!(
        "thread_panic payload:{} curr_thread_name:{} location:{} backtrace:{:?}",
        payload,
        curr_thread_name,
        location,
        backtrace::Backtrace::new()
    );

    error!(
        "thread_panic payload:{} curr_thread_name:{} location:{} backtrace:{:?}",
        payload,
        curr_thread_name,
        location,
        backtrace::Backtrace::new()
    );

    std::process::exit(1);
}
