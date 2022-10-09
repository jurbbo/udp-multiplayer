use crate::requests::jobtype::get_job_single_byte;
use crate::requests::{Job, JobType};
use std::time::Duration;
use std::time::Instant;

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
            handle: handle,
            start_instant: Instant::now(),
            job_type: job_type,
            raw_data: byte_array,
            pending: true,
            requested_count: 0,
            job_should_complite: job_should_complite,
        }
    }

    pub fn get_handle(&self) -> u8 {
        self.handle
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
