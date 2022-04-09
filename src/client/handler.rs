use crate::client::Arc;
use crate::client::Client;
use crate::client::Mutex;
use crate::requests::jobs::Jobs;
use crate::requests::jobtype::JobType;

struct SocketListener {
    jobs: Arc<Mutex<Jobs>>,
    time_to_die: Arc<AtomicBool>,
    socket: Arc::new(socket),
}

impl SocketListener {
    pub fn new(jobs: Arc<Mutex<Jobs>>, socket: Arc<UdpSocket>) -> SocketListener {
        SocketListener {
            jobs: jobs,
            socket: socket,
            time_to_die: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn listen(&self) {}

    fn handle_received_data(&self, handle: u8, job_type: JobType, raw_data: &mut [u8]) {
        match job_type {
            JobType::DATA_PUSH_ACTION => {
                let mut jobs_changer = jobs.lock().unwrap();
                let job_maybe = (*jobs_changer).jobs.get_mut(&handle);

                // ON SUCCESS!
                if job_maybe.is_some() {
                    let job = job_maybe.unwrap();
                    let job_duration = job.finish();
                    (*jobs_changer).add_finish_time(job_duration);

                    /*
                    println!(
                        "data {}, length {}, duration micros {}",
                        filled_buf[0],
                        number_of_bytes,
                        job_duration.as_micros()
                    );
                    */
                } else {
                    (*jobs_changer).failed += 1;
                }
            }
            JobType::DATA_PUSH_RECEIVED => {
                let player = raw_data[2];
            }
        }
    }
}
