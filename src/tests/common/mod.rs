use crate::tests::common::testcounter::global_test_counter;
use crate::tests::common::testserver::{global_test_server, wait_till_server_is_running};
use std::thread;
use std::time::Duration;

pub mod testclient;
pub mod testcounter;
pub mod testserver;

pub fn init_test_environment() {
    global_test_server();
    wait_till_server_is_running();
    global_test_counter();
}

pub fn pause(time: u64) {
    let ten_millis = Duration::from_millis(time);
    thread::sleep(ten_millis);
}
