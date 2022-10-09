use crate::requests::Job;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

pub struct Jobs {
    pub jobs: Arc<Mutex<HashMap<u8, Job>>>,
    next_job_handle: Mutex<u8>,
    job_finish_times: Mutex<VecDeque<f64>>,
    job_finish_time_average: Mutex<f64>,
    packages_handled: Mutex<u128>,
    packages_failed: Mutex<u128>,
    average_set_instant: Mutex<Instant>,
}

impl Jobs {
    pub fn new() -> Jobs {
        Jobs {
            jobs: Arc::new(Mutex::new(HashMap::<u8, Job>::new())),
            next_job_handle: Mutex::new(0),
            job_finish_times: Mutex::new(VecDeque::new()),
            job_finish_time_average: Mutex::new(500.0),
            packages_handled: Mutex::new(0),
            packages_failed: Mutex::new(0),
            average_set_instant: Mutex::new(Instant::now()),
        }
    }

    pub fn get_ping(&self) -> f64 {
        let job_finish_time_average_changer = self.job_finish_time_average.lock().unwrap();
        *job_finish_time_average_changer
    }

    pub fn add_packages_failed(&self) {
        let mut packages_failed_changer = self.packages_failed.lock().unwrap();
        (*packages_failed_changer) += 1;
    }

    pub fn get_job_finish_time_average(&self) -> f64 {
        let job_finish_time_average_changer = self.job_finish_time_average.lock().unwrap();
        *job_finish_time_average_changer
    }

    pub fn add_packages_handled(&self) {
        let mut packages_handled_changer = self.packages_handled.lock().unwrap();
        (*packages_handled_changer) += 1;
    }

    pub fn insert_job(&self, handle: u8, job: Job) {
        let mut jobs_changer = self.jobs.lock().unwrap();
        (*jobs_changer).insert(handle, job);
        println!("{}", (*jobs_changer).len());
    }

    pub fn remove_job(&self, handle: u8) {
        let job_duration_maybe = self.get_job_duration(handle);
        match job_duration_maybe {
            None => {}
            Some(job_duration) => {
                self.add_finish_time(job_duration);
            }
        }
        let mut jobs_changer = self.jobs.lock().unwrap();
        (*jobs_changer).remove(&handle);
    }

    /// fns related to ping counting,
    /// --> job durations are collected to queue,
    /// --> average is calculated in every 10 secs

    fn get_job_duration(&self, handle: u8) -> Option<Duration> {
        let mut jobs_changer = self.jobs.lock().unwrap();
        let job_maybe = jobs_changer.get_mut(&handle);
        // Job is not found, something went wrong.
        if job_maybe.is_none() {
            self.add_packages_failed();
            // TO DO ERROR STATE MUST BE RAISED!
            //let mut events_changer = self.events.lock().unwrap();
            //(*events_changer).on_error();
            return None;
        }
        let job = job_maybe.unwrap();
        Some(job.finish())
    }

    fn push_job_duration_to_queue(&self, duration: Duration) {
        let mut job_finish_times_changer = self.job_finish_times.lock().unwrap();

        (*job_finish_times_changer).push_back((duration.as_micros() as f64) / 1000.0);
        if (*job_finish_times_changer).len() > 200 {
            (*job_finish_times_changer).pop_front();
        }
    }

    fn has_10_secs_passed_since_last_average_calucation(&self) -> bool {
        let average_set_instant_changer = self.average_set_instant.lock().unwrap();
        let now = Instant::now();
        let last_average_set_secs_ago =
            (now.duration_since(*average_set_instant_changer)).as_secs();

        last_average_set_secs_ago > 9
    }

    fn set_instant_as_now(&self) {
        let mut average_set_instant_changer = self.average_set_instant.lock().unwrap();
        *average_set_instant_changer = Instant::now();
    }

    pub fn add_finish_time(&self, duration: Duration) {
        // Jobs collects job durations to calculate
        self.push_job_duration_to_queue(duration);

        // set average every ten seconds
        if self.has_10_secs_passed_since_last_average_calucation() {
            self.set_instant_as_now();
            self.set_average_finish_time();
        }
    }

    pub fn get_next_job_handle(&self) -> u8 {
        let mut next_job_handle_changer = self.next_job_handle.lock().unwrap();
        let next_handle_before_addition = *next_job_handle_changer;
        if *next_job_handle_changer < 255 {
            *next_job_handle_changer += 1;
        } else {
            *next_job_handle_changer = 0;
        }
        next_handle_before_addition
    }

    fn set_average_finish_time(&self) {
        // if there is less than 5 instances of jobs times, return 500 millis just to be sure.
        let job_finnish_times_changer = self.job_finish_times.lock().unwrap();

        if (*job_finnish_times_changer).len() < 5 {
            *(self.job_finish_time_average.lock().unwrap()) = 500.0;
            return;
        }
        let mut calculated: f64 = 0.0;
        for job_finish_time in (*job_finnish_times_changer).iter() {
            calculated += job_finish_time;
        }
        let average = calculated / ((*job_finnish_times_changer).len() as f64);
        println!(
            "calulated average ping {}, packages handled {}, failed {}",
            average,
            *(self.packages_handled.lock().unwrap()),
            *(self.packages_failed.lock().unwrap())
        );

        // if under 1, use 10.0, and do not use over 500.0 ms average.
        *(self.job_finish_time_average.lock().unwrap()) = if average <= 1.0 {
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
