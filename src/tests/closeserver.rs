use crate::tests::common::init_test_environment;
use crate::tests::common::pause;
use crate::tests::common::testcounter::are_all_tests_complited;
use crate::tests::common::testserver::close_server;
use crate::tests::common::testserver::is_server_running;

#[test]
fn close_global_test_server() {
    init_test_environment();

    while !are_all_tests_complited() {
        pause(100);
    }

    println!("All tests are ran... closing server.");

    let _is_running = close_server();
    let mut is_closed = false;
    for _i in 1..100 {
        pause(100);
        if !is_server_running() {
            is_closed = true;
            break;
        }
    }
    if !is_closed {
        println!("Server refuse to close in 10 secs...");
    }
    assert_eq!(true, is_closed);
}
