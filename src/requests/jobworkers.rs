use crate::requests::jobs::Jobs;
use crate::requests::Job;
use crate::requests::JobAction;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::Instant;

/*
 * Init job handler
 *
 * Job handler checks if there is problematic jobs
 *
 * Static function that takes jobs.jobs aka Arc<Mutex<HashMap<u8, Job>>> as first parameter.
 */
pub fn run_job_handler(
    jobs: Arc<Jobs>,
    socket_send_channel_tx: Sender<(Vec<u8>, Job)>,
    time_to_die: Arc<AtomicBool>,
) {
    loop {
        thread::sleep(Duration::from_millis(10));

        // break if closing...
        if time_to_die.load(Ordering::SeqCst) {
            break;
        }

        let mut jobs_changer = jobs.jobs.lock().unwrap();
        let now = Instant::now();
        let mut failed_job_indexes = Vec::<u8>::new();
        for (job_handle, job) in &mut *jobs_changer {
            if job.is_pending_request_late(now) {
                if job.requested_count < 10 {
                    // resend failed message
                    println!("PACKED FAILED. RESEND NUMBER {}!", job.requested_count);
                    job.reset_start_instant();
                    job.requested_count += 1;
                    //let result = socket.send(&job.raw_data);
                    let result = socket_send_channel_tx.send((job.raw_data.clone(), job.clone()));
                    if result.is_err() {
                        panic!("channel error state");
                    }
                    // if connection fails, remove job.
                    if result.is_err() {
                        job.pending = false;
                        failed_job_indexes.push(*job_handle);
                    }
                // Too many retryes, let's cancel the job.
                } else {
                    job.pending = false;
                    failed_job_indexes.push(*job_handle);
                }
            }
        }
        // remove failed jobs from index
        for failed_job_index in failed_job_indexes {
            println!("index remover {}", failed_job_index);
            (*jobs_changer).remove(&failed_job_index);
        }
    }
}

/*
 * Init job channel consumer
 *
 * Static function that takes Arc<Jobs> as first parameter.
 */
pub fn run_job_channel_consumer(
    jobs: Arc<Jobs>,
    job_channel_rx: Receiver<(JobAction, u8, Option<Job>)>,
    time_to_die: Arc<AtomicBool>,
) {
    loop {
        // break loop if server is closing...
        if time_to_die.load(Ordering::Relaxed) {
            break;
        }

        let result = job_channel_rx.recv();
        match result {
            Err(e) => {
                println!("Job handler consumer failed: {}", e);
            }
            Ok(data) => {
                // tuple of job_action, job_index, and optional job
                let (job_action, job_handle, job_maybe) = data;

                match job_action {
                    JobAction::ADD => {
                        match job_maybe {
                            None => {}
                            Some(job) => {
                                // job handle is created with next_job_handle fn,
                                // when job instance is created
                                jobs.insert_job(job_handle, job);
                            }
                        }
                    }
                    JobAction::REMOVE => {
                        jobs.remove_job(job_handle);
                    }
                    JobAction::INCFAILED => {
                        jobs.add_packages_failed();
                    }
                    JobAction::INCHANDLED => {
                        jobs.add_packages_handled();
                    }
                }
            }
        }
    }
}
