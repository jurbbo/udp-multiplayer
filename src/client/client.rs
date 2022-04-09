use crate::client::socketlistener::SocketListener;
use crate::client::RequestEvents;
use crate::requests::jobs::Job;
use crate::requests::jobs::Jobs;
use crate::requests::jobtype::{tranfer_job_type_from_number, JobType};
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::Instant;

trait SendRequest {
    fn send_request(&self, job_type: JobType, raw_data: &mut Vec<u8>);
}

pub struct Client {
    jobs: Arc<Mutex<Jobs>>,
    time_to_die: Arc<AtomicBool>,
    thread_handles: Option<Vec<JoinHandle<()>>>,
    socket: Arc<UdpSocket>,
    threads_count: u8,
    handle_data_cb: Arc<Mutex<fn(job_type: JobType, raw_data: &mut [u8])>>,
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
            thread_handles: None,
            socket: Arc::new(socket),
            threads_count: threads_count,
            handle_data_cb: Arc::new(Mutex::new(handle_data_cb)),
        }
    }

    pub fn die(&mut self) {
        self.time_to_die.store(true, Ordering::SeqCst);

        let handles = self.thread_handles.take();
        if handles.is_none() {
            return;
        }
        handles.unwrap().into_iter().for_each(|handle| {
            handle.join().unwrap();
        });
    }

    pub fn run<S: 'static>(&mut self, events: Arc<Mutex<S>>)
    where
        S: RequestEvents + Send + Sync,
    {
        self.init_job_handler();
        self.init_listeners(events);
    }

    pub fn get_job_ping(&self) -> f64 {
        let jobs_changer = self.jobs.lock().unwrap();
        (*jobs_changer).job_finish_time_average
    }

    // Listener thread workers initialization
    fn init_listeners<S: 'static>(&mut self, events: Arc<Mutex<S>>)
    where
        S: RequestEvents + Send + Sync,
    {
        //let events_sharable = Arc::new(events);
        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        for _handle_index in 1..self.threads_count {
            // clone structs for next thread
            let jobs = Arc::clone(&self.jobs);
            let socket = Arc::clone(&self.socket);
            let time_to_die = Arc::clone(&self.time_to_die);
            let events = Arc::clone(&events);
            //let handle_data_cb = Arc::clone(&self.handle_data_cb);

            // worker for listening server data starts here
            let handle = thread::spawn(move || {
                (SocketListener::new(&jobs, &socket, &time_to_die, &events)).init_listener()
            });

            handles.push(handle);
        }
        self.thread_handles = Some(handles);
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
