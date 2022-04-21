use crate::client::socketlistener::SocketListener;
use crate::client::RequestEvents;
use crate::requests::jobs::Job;
use crate::requests::jobs::Jobs;
use crate::requests::jobtype::JobType;
use crate::requests::jobtype::{ClientJob, ServerJob};
use std::io::Error;
use std::io::ErrorKind;
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
    error_state_previous: Arc<AtomicBool>,
    error_state_current: Arc<AtomicBool>,
    error_state_start_time: Option<Instant>,
    thread_handles: Option<Vec<JoinHandle<()>>>,
    socket: Option<Arc<UdpSocket>>,
    threads_count: u8,
    //handle_data_cb: Arc<Mutex<fn(job_type: JobType, raw_data: &mut [u8])>>,
}

impl Client {
    pub fn new(
        threads_count: u8,
        //handle_data_cb: fn(job_type: JobType, raw_data: &mut [u8]),
    ) -> Client {
        Client {
            jobs: Arc::new(Mutex::new(Jobs::new())),
            time_to_die: Arc::new(AtomicBool::new(false)),
            error_state_previous: Arc::new(AtomicBool::new(false)),
            error_state_current: Arc::new(AtomicBool::new(false)),
            error_state_start_time: None,
            thread_handles: None,
            socket: None,
            threads_count: threads_count,
            //handle_data_cb: Arc::new(Mutex::new(handle_data_cb)),
        }
    }

    pub fn connect(&mut self, local_ip: String, server_ip: String) -> Result<(), std::io::Error> {
        let socket_result = UdpSocket::bind(local_ip);
        if socket_result.is_err() {
            let err = socket_result.unwrap_err();
            return Err(err);
        }

        let socket = socket_result.unwrap();
        let connect_result = socket.connect(server_ip);

        match connect_result {
            Ok(()) => {
                self.socket = Some(Arc::new(socket));
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    // Takes ownership of thread handles and joins threads.
    // UDP listenings and job handling ends.
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

    pub fn get_job_ping(&self) -> f64 {
        let jobs_changer = self.jobs.lock().unwrap();
        (*jobs_changer).job_finish_time_average
    }

    pub fn is_in_error_state(&self) -> bool {
        self.error_state_current.load(Ordering::SeqCst)
    }

    // Listener thread workers initialization
    pub fn init_listeners<S: 'static>(&mut self, events: Arc<Mutex<S>>)
    where
        S: RequestEvents + Send + Sync,
    {
        if self.socket.is_none() {
            return;
        }

        let socket = self.socket.as_ref().unwrap();

        //let events_sharable = Arc::new(events);
        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        for _handle_index in 1..self.threads_count {
            // clone structs for next thread
            let jobs = Arc::clone(&self.jobs);
            let socket = Arc::clone(&socket);
            let time_to_die = Arc::clone(&self.time_to_die);
            let events = Arc::clone(&events);
            let error_state_current = Arc::clone(&self.error_state_current);
            let error_state_previous = Arc::clone(&self.error_state_previous);

            //let handle_data_cb = Arc::clone(&self.handle_data_cb);

            // worker for listening server data starts here
            let handle = thread::spawn(move || {
                (SocketListener::new(
                    &jobs,
                    &socket,
                    &time_to_die,
                    &events,
                    &error_state_current,
                    &error_state_previous,
                ))
                .init_listener()
            });

            handles.push(handle);
        }
        self.push_handles(handles);
    }

    // Job handler checks if there is problematic jobs
    pub fn init_job_handler(&mut self) {
        if self.socket.is_none() {
            return;
        }
        let socket = self.socket.as_ref().unwrap();

        let socket = Arc::clone(&socket);
        let jobs = Arc::clone(&self.jobs);
        let time_to_die = Arc::clone(&self.time_to_die);
        // worker for reading server answer starts here
        let mut handle = vec![thread::spawn(move || loop {
            // break if server is closing...
            thread::sleep(Duration::from_micros(1000));
            if time_to_die.load(Ordering::SeqCst) {
                break;
            }

            let mut jobs_changer = jobs.lock().unwrap();
            let now = Instant::now();
            let mut failed_job_indexes = Vec::<u8>::new();
            for (job_index, job) in &mut (*jobs_changer).jobs {
                if job.is_pending_request_late(now) {
                    if job.requested_count < 10 {
                        // resend failed message
                        println!("PACKED FAILED. RESEND NUMBER {}!", job.requested_count);
                        job.reset_start_instant();
                        job.requested_count += 1;
                        let result = socket.send(&job.raw_data);
                        // if connection fails, remove job.
                        if result.is_err() {
                            job.pending = false;
                            failed_job_indexes.push(*job_index);
                        }
                    // Too many retryes, let's cancel the job.
                    } else {
                        job.pending = false;
                        failed_job_indexes.push(*job_index);
                    }
                }
            }
            // remove failed jobs from index
            for failed_job_index in failed_job_indexes {
                println!("index remover {}", failed_job_index);
                (*jobs_changer).jobs.remove(&failed_job_index);
            }
        })];
        self.push_handles(handle);
    }

    fn push_handles(&mut self, mut handles_to_append: Vec<JoinHandle<()>>) {
        if self.thread_handles.is_none() {
            self.thread_handles = Some(handles_to_append);
        } else {
            let mut handles = self.thread_handles.take().unwrap();
            handles.append(&mut handles_to_append);
            self.thread_handles = Some(handles);
        }
    }

    pub fn send_request(
        &self,
        client_job_type: ClientJob,
        raw_data: &mut Vec<u8>,
    ) -> Result<(), std::io::Error> {
        if self.socket.is_none() {
            return Err(Error::new(
                ErrorKind::Other,
                "Cannot send without activated socket. Hint: has port or ip address failed?",
            ));
        }
        let socket = self.socket.as_ref().unwrap();

        // Create job to follow up server answer.
        let mut jobs_changer = self.jobs.lock().unwrap();
        let next_index = (*jobs_changer).get_next_index();
        let job_should_complite = ((*jobs_changer).job_finish_time_average * 2.0) as u128;
        let job_type: JobType = (ServerJob::NoServerAction, client_job_type);
        let job = Job::new(next_index, job_type, raw_data, job_should_complite);

        let data = job.get_raw_data();

        // Job is inserted to Jobs
        (*jobs_changer).jobs.insert(next_index, job);

        let result = socket.send(&data);
        if result.is_err() {
            // let's remove just created job when there's an error.
            (*jobs_changer).jobs.remove(&next_index);

            return Err(Error::new(
                ErrorKind::Other,
                "UDP packet send failed. Connection problem?",
            ));
        }

        Ok(())
    }
}
