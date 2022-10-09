use crate::tests::TESTCOUNT;
use std::sync::Mutex;
use std::sync::Once;

static mut COUNTER: Option<Mutex<TestCounter>> = None;
static INITCOUNTER: Once = Once::new();

pub struct TestCounter {
    total: usize,
    finished: usize,
}

impl TestCounter {
    pub fn new(total: usize) -> TestCounter {
        TestCounter {
            total: total,
            finished: 0,
        }
    }
    pub fn add_finished_count(&mut self) {
        self.finished += 1;
    }

    pub fn are_all_tests_complited(&self) -> bool {
        self.finished >= self.total
    }
}

pub fn global_test_counter<'a>() -> &'a Mutex<TestCounter> {
    INITCOUNTER.call_once(|| unsafe {
        COUNTER = Some(Mutex::new(TestCounter::new(TESTCOUNT)));
    });
    // As long as this function is the only place with access to the static variable,
    // giving out a read-only borrow here is safe because it is guaranteed no more mutable
    // references will exist at this point or in the future.
    unsafe { COUNTER.as_ref().unwrap() }
}

pub fn add_finished_count() {
    global_test_counter().lock().unwrap().add_finished_count();
}
pub fn are_all_tests_complited() -> bool {
    global_test_counter().lock().unwrap().are_all_tests_complited()
}
