use crate::requests::jobs::Job;
use crate::requests::jobs::Jobs;
use crate::requests::jobtype::{tranfer_job_type_from_number, JobType};
use std::net::UdpSocket;
use std::ops::DerefMut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::Instant;

pub trait RequestEvents {
    fn handle_data(&mut self, job_type: JobType, raw_data: &mut [u8]);
    fn handle_ping(&mut self, interval: Instant);
}

pub struct Client {
    jobs: Arc<Mutex<Jobs>>,
    time_to_die: Arc<AtomicBool>,
    thread_handles: Vec<JoinHandle<()>>,
    socket: Arc<UdpSocket>,
    threads_count: u8,
    handle_data_cb: Arc<Mutex<fn(job_type: JobType, raw_data: &mut [u8])>>,
}

/*
trait Handler {
    fn handle_received_data(
        handle: u8,
        job_type: JobType,
        raw_data: &mut [u8],
        jobs: Arc<Mutex<Jobs>>,
    );
}
*/
/*
impl RequestEvents for Client {
    fn handle_data(&self, job_type: JobType, raw_data: &mut [u8]) {
        println!("Received..");
    }
    fn handle_ping(&self, interval: Instant) {
        println!("Ping");
    }
}
*/

pub fn run<S: 'static, E>(mut client: Client, mut events: S)
where
    S: RequestEvents,
    E: std::error::Error,
{
    client.run();
}

impl Client {
    pub fn new(
        local_ip: &str,
        server_ip: &str,
        threads_count: u8,
        handle_data_cb: fn(job_type: JobType, raw_data: &mut [u8]),
    ) -> Client {
        let socket = UdpSocket::bind(local_ip).expect("couldn't bind to address");
        socket.connect(server_ip).expect("Connect failed");

        Client {
            jobs: Arc::new(Mutex::new(Jobs::new())),
            time_to_die: Arc::new(AtomicBool::new(false)),
            thread_handles: vec![],
            socket: Arc::new(socket),
            threads_count: threads_count,
            handle_data_cb: Arc::new(Mutex::new(handle_data_cb)),
        }
    }

    pub fn die(&mut self) {
        self.time_to_die.store(true, Ordering::SeqCst);
        for handle in &self.thread_handles {
            //handle.join().unwrap();
        }
    }

    pub fn runn<S: 'static>(&mut self, events: Arc<Mutex<S>>)
    where
        S: RequestEvents + std::marker::Send + std::marker::Sync,
    {
        self.init_job_handler();
        self.init_listeners(events);
    }

    pub fn run(&mut self) {
        self.init_job_handler();
        //self.init_listeners();
    }

    pub fn get_job_ping(&self) -> f64 {
        let jobs_changer = self.jobs.lock().unwrap();
        (*jobs_changer).job_finish_time_average
    }

    // UDP listener workers init
    fn init_listeners<S: 'static>(&mut self, events: Arc<Mutex<S>>)
    where
        S: RequestEvents + Send + Sync,
    {
        let events_sharable = Arc::new(events);
        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        for _handle_index in 1..self.threads_count {
            let jobs = Arc::clone(&self.jobs);
            let socket = Arc::clone(&self.socket);
            let time_to_die = Arc::clone(&self.time_to_die);
            let events = Arc::clone(&events_sharable);
            let handle_data_cb = Arc::clone(&self.handle_data_cb);

            // worker for reading server answer starts here
            let handle = thread::spawn(move || loop {
                // break loop if server is closing...
                if time_to_die.load(Ordering::SeqCst) {
                    break;
                }

                let mut buf = [0; 10];
                let (number_of_bytes, _src_addr) = socket
                    .recv_from(&mut buf)
                    .expect("Socket failed on receive");
                let raw_data = &mut buf[..number_of_bytes];

                // data is received, lets work on it.
                let handle = raw_data[0];
                let job_type = tranfer_job_type_from_number(&raw_data[1]);

                if job_type.is_none() {
                    let mut jobs_changer = jobs.lock().unwrap();
                    (*jobs_changer).packages_failed += 1;
                    continue;
                }

                match job_type.unwrap() {
                    JobType::DATA_PUSH_ACTION => {
                        let mut jobs_changer = jobs.lock().unwrap();
                        let job_maybe = (*jobs_changer).jobs.get_mut(&handle);
                        // ON SUCCESS!
                        if job_maybe.is_some() {
                            let job = job_maybe.unwrap();
                            let job_duration = job.finish();
                            (*jobs_changer).add_finish_time(job_duration);
                            (*jobs_changer).packages_handled += 1;
                            let handle_data_cb_unlocked = handle_data_cb.lock().unwrap();
                            (*handle_data_cb_unlocked)(JobType::DATA_PUSH_ACTION, raw_data);
                            let mut events_changer = events.lock().unwrap();
                            (*events_changer).handle_data(JobType::DATA_PUSH_ACTION, raw_data);

                            /*
                            println!(
                                "data {}, length {}, duration micros {}",
                                filled_buf[0],
                                number_of_bytes,
                                job_duration.as_micros()
                            );
                            */
                        } else {
                            (*jobs_changer).packages_failed += 1;
                        }
                    }
                    JobType::DATA_PUSH_RECEIVED => {
                        let player = raw_data[2];
                    }
                    __ => {}
                }
            });
            handles.push(handle);
        }
        self.thread_handles = handles;
    }

    // Job handler checks if there is problematic jobs
    fn init_job_handler(&self) -> Vec<JoinHandle<()>> {
        let socket = Arc::clone(&self.socket);
        let jobs = Arc::clone(&self.jobs);
        let time_to_die = Arc::clone(&self.time_to_die);
        // worker for reading server answer starts here
        vec![thread::spawn(move || loop {
            // break if server is closing...
            thread::sleep(Duration::from_micros(1000));
            if time_to_die.load(Ordering::SeqCst) {
                break;
            }

            let mut jobs_changer = jobs.lock().unwrap();
            let now = Instant::now();
            let mut job_indexes_failed = Vec::<u8>::new();
            for (job_index, job) in &mut (*jobs_changer).jobs {
                if job.is_pending_request_late(now) {
                    if job.requested_count < 10 {
                        // resend failed message
                        println!("PACKED FAILED. RESEND NUMBER {}!", job.requested_count);
                        job.reset_start_instant();
                        job.requested_count += 1;
                        socket.send(&job.raw_data).expect("couldn't send message");
                    } else {
                        job.pending = false;
                        job_indexes_failed.push(*job_index);
                    }
                }
            }
            // remove failed jobs from index
            for job_index_failed in job_indexes_failed {
                (*jobs_changer).jobs.remove(&job_index_failed);
            }
        })]
    }

    pub fn send_request(&self, job_type: JobType, raw_data: &mut Vec<u8>) {
        let mut jobs_changer = self.jobs.lock().unwrap();
        let next_index = (*jobs_changer).get_next_index();
        let job_should_complite = ((*jobs_changer).job_finish_time_average * 2.0) as u128;
        let job = Job::new(next_index, job_type, raw_data, job_should_complite);

        let data = job.get_raw_data();

        // Job is inserted to Jobs
        (*jobs_changer).jobs.insert(next_index, job);

        self.socket.send(&data).expect("couldn't send message");
    }
}
