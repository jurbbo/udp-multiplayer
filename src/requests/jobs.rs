use crate::requests::jobtype::{get_job_single_byte, JobType};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::Duration;
use std::time::Instant;

pub struct Job {
    pub start_instant: Instant,
    pub pending: bool,
    pub raw_data: Vec<u8>,
    pub job_type: JobType,
    pub requested_count: i8,
    job_should_complite: u128,
}

impl Job {
    pub fn new(
        handle: u8,
        job_type: JobType,
        raw_data: &mut Vec<u8>,
        job_should_complite: u128,
    ) -> Job {
        // raw_data array will be created
        // first byte is job handle, second byte is jobtype, rest is user data
        let mut byte_array = vec![handle, get_job_single_byte(&job_type)];
        byte_array.append(raw_data);

        Job {
            start_instant: Instant::now(),
            job_type: job_type,
            raw_data: byte_array,
            pending: true,
            requested_count: 0,
            job_should_complite: job_should_complite,
        }
    }

    pub fn reset_start_instant(&mut self) {
        self.start_instant = Instant::now();
    }

    pub fn finish(&mut self) -> Duration {
        let now = Instant::now();
        self.pending = false;
        now.duration_since(self.start_instant)
    }
    pub fn get_raw_data(&self) -> Vec<u8> {
        self.raw_data.to_vec()
    }

    pub fn is_pending_request_late(&self, now: Instant) -> bool {
        // only check for pending requests
        if !self.pending {
            return false;
        }

        let job_duration = now.duration_since(self.start_instant);
        if job_duration.as_millis() > self.job_should_complite {
            return true;
        } else {
            return false;
        }
    }
}

pub struct Jobs {
    pub jobs: HashMap<u8, Job>,
    pub next_index: u8,
    job_finish_times: VecDeque<f64>,
    pub job_finish_time_average: f64,
    pub packages_handled: u128,
    pub packages_failed: u128,
    average_set_instant: Instant,
}

impl Jobs {
    pub fn new() -> Jobs {
        Jobs {
            jobs: HashMap::<u8, Job>::new(),
            next_index: 0,
            job_finish_times: VecDeque::new(),
            job_finish_time_average: 500.0,
            packages_handled: 0,
            packages_failed: 0,
            average_set_instant: Instant::now(),
        }
    }

    pub fn add_finish_time(&mut self, duration: Duration) {
        self.job_finish_times
            .push_back((duration.as_micros() as f64) / 1000.0);
        if self.job_finish_times.len() > 200 {
            self.job_finish_times.pop_front();
        }
        let now = Instant::now();
        let last_average_set_secs_ago = (now.duration_since(self.average_set_instant)).as_secs();
        // set average every ten seconds
        if last_average_set_secs_ago > 9 {
            self.average_set_instant = Instant::now();
            self.set_average_finish_time();
        }
    }

    pub fn get_next_index(&mut self) -> u8 {
        let next_handle = self.next_index;
        if self.next_index < 255 {
            self.next_index += 1;
        } else {
            self.next_index = 0;
        }
        next_handle
    }

    fn set_average_finish_time(&mut self) {
        // if there is less than 5 instances of jobs times, return 500 millis just to be sure.
        if self.job_finish_times.len() < 5 {
            self.job_finish_time_average = 500.0;
            return;
        }
        let mut calculated: f64 = 0.0;
        for job_finish_time in self.job_finish_times.iter() {
            calculated += job_finish_time;
        }
        let average = calculated / (self.job_finish_times.len() as f64);
        println!(
            "calulated average ping {}, packages handled {}, failed {}",
            average, self.packages_handled, self.packages_failed
        );

        // if under 1, use 10.0, and do not use over 500.0 ms average.
        self.job_finish_time_average = if average <= 1.0 {
            10.0
        } else {
            if average < 500.0 {
                average
            } else {
                500.0
            }
        };
    }
}
