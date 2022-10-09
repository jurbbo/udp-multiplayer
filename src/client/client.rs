use crate::client::socketlistener::SocketListener;
use crate::client::socketsender::SocketSender;
use crate::client::RequestEvents;
use crate::helpers::threadkiller::client_channel_killer;
use crate::helpers::threadkiller::thread_killer;
use crate::protocol::Protocol;
use crate::requests::jobs::Jobs;
use crate::requests::jobworkers;
use crate::requests::{ClientJob, Job, JobAction, JobType, ServerJob};
use crate::socket::SocketCombatible;
use std::io::Error;
use std::io::ErrorKind;
use std::net::IpAddr;
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::thread::JoinHandle;
use std::time::Instant;

/*
trait SendRequest {
    fn send_request(&self, job_type: JobType, raw_data: &mut Vec<u8>);
}
*/

pub struct Client {
    // Multi transmitets txs for threads to share.
    // Ownership of receivers is stored at init and then pushed to handler threads.
    job_action_channel_tx: Option<Sender<(JobAction, u8, Option<Job>)>>,
    job_action_channel_rx: Option<Receiver<(JobAction, u8, Option<Job>)>>,

    socket_send_channel_tx: Option<Sender<(Vec<u8>, Job)>>,
    socket_send_channel_rx: Option<Receiver<(Vec<u8>, Job)>>,

    _socket_receive_channel_tx: Option<Sender<Vec<u8>>>,

    jobs: Arc<Jobs>,
    // Time_to_die variable to terminate threads.
    time_to_die: Arc<AtomicBool>,
    // Is running implicates that socket is tied to address, and socket listener is activated.
    is_running: Arc<AtomicBool>,
    // Client IP
    ip: Option<IpAddr>,
    // Client port
    port: Option<u16>,
    // Default and custom protocols. If defaults are missing or mutated, client might fail.
    protocols: Arc<Protocol>,
    error_state_previous: Arc<AtomicBool>,
    error_state_current: Arc<AtomicBool>,
    error_state_start_time: Option<Instant>,
    thread_handles: Option<Vec<JoinHandle<()>>>,
    socket: Option<Arc<UdpSocket>>,
    threads_count: u8,
    //handle_data_cb: Arc<Mutex<fn(job_type: JobType, raw_data: &mut [u8])>>,
}

impl SocketCombatible for Client {
    fn get_port(&self) -> Option<u16> {
        self.port
    }
    fn get_ip(&self) -> Option<IpAddr> {
        self.ip
    }
    fn is_running(&self) -> bool {
        return self.is_running.load(Ordering::SeqCst);
    }
    fn get_is_running_atomic(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.is_running)
    }
}

impl Client {
    pub fn new(
        threads_count: u8,
        //handle_data_cb: fn(job_type: JobType, raw_data: &mut [u8]),
    ) -> Client {
        Client {
            // channels for thread to thread communication.
            // constructor formats them as none, since channels
            // are created in init functions.
            socket_send_channel_tx: None,
            socket_send_channel_rx: None,

            _socket_receive_channel_tx: None,
            job_action_channel_tx: None,
            job_action_channel_rx: None,

            // job data struct
            jobs: Arc::new(Jobs::new()),
            time_to_die: Arc::new(AtomicBool::new(false)),
            is_running: Arc::new(AtomicBool::new(false)),
            ip: None,
            port: None,
            protocols: Arc::new(Protocol::new()),
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
        let socket = UdpSocket::bind(local_ip)?;
        let socket_addr = socket.local_addr()?;
        self.ip = Some(socket_addr.ip());
        self.port = Some(socket_addr.port());
        let connect_result = socket.connect(server_ip);

        match connect_result {
            Ok(()) => {
                self.socket = Some(Arc::new(socket));
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub fn init_channels(&mut self) {
        // Creation of Socket Send Channel
        let (socket_send_channel_tx, socket_send_channel_rx) = channel::<(Vec<u8>, Job)>();
        self.socket_send_channel_tx = Some(socket_send_channel_tx);
        self.socket_send_channel_rx = Some(socket_send_channel_rx);

        // Creation of Job Handler Action Channel
        let (job_handler_channel_tx, job_handler_channel_rx) =
            channel::<(JobAction, u8, Option<Job>)>();
        self.job_action_channel_tx = Some(job_handler_channel_tx);
        self.job_action_channel_rx = Some(job_handler_channel_rx);
    }

    // Takes ownership of thread handles and joins threads.
    // UDP listening and job handling ends. Uses thread killer from helpers
    // to activate blocking channel receivers and UDP listener.
    pub fn die(&mut self) -> bool {
        println!(
            "Closing client running on {}:{}",
            self.ip.unwrap(),
            self.port.unwrap()
        );
        if self.job_action_channel_tx.is_none() || self.socket_send_channel_tx.is_none() {
            panic!("Closing client, but channels are not activated. This should not be possible???!?!?");
        }

        /*
        let job_channel_tx = self.job_action_channel_tx.as_ref().unwrap().clone();
        let socket_send_channel_tx = self.socket_send_channel_tx.as_ref().unwrap().clone();
        let is_running = self.get_is_running_atomic().clone();

        let channel_killer_handle =
            client_channel_killer(job_channel_tx, socket_send_channel_tx, is_running);
        */
        match thread_killer(self) {
            None => return false,
            Some(thread_killer_handle) => match self.thread_handles.take() {
                None => return true,
                Some(handles) => {
                    self.time_to_die.store(true, Ordering::SeqCst);

                    handles.into_iter().for_each(|handle| {
                        if handle.thread().name().unwrap() == "Listener thread" {
                            print!("Closing '{}' ...", handle.thread().name().unwrap());
                            handle.join().unwrap();
                            println!("success.")
                        }
                    });
                    self.is_running.store(false, Ordering::SeqCst);
                    print!("Closing helpers: Thread killer, ");
                    thread_killer_handle.join().unwrap();
                    //println!("channel killer.");
                    //channel_killer_handle.unwrap().join().unwrap();
                    return true;
                }
            },
        }
    }

    pub fn get_job_ping(&self) -> f64 {
        self.jobs.get_ping()
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

        if self.job_action_channel_tx.is_none() {
            return;
        }

        let socket = self.socket.as_ref().unwrap();

        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        // clone stuff for next thread
        let socket = Arc::clone(&socket);
        let protocols = Arc::clone(&self.protocols);
        let time_to_die = Arc::clone(&self.time_to_die);
        let events = Arc::clone(&events);
        let error_state_current = Arc::clone(&self.error_state_current);
        let error_state_previous = Arc::clone(&self.error_state_previous);
        let job_channel_tx = self.job_action_channel_tx.as_ref().unwrap().clone();

        //let handle_data_cb = Arc::clone(&self.handle_data_cb);

        // worker for listening server data starts here
        let listener_thread = thread::Builder::new().name(format!("Listener thread"));
        match listener_thread.spawn(move || {
            (SocketListener::new(
                job_channel_tx,
                socket,
                protocols,
                time_to_die,
                events,
                error_state_current,
                error_state_previous,
            ))
            .init_listener()
        }) {
            Ok(handle) => handles.push(handle),
            Err(e) => {
                println!("{}", e);
            }
        }
        self.push_handles(handles);
    }

    pub fn mark_client_start(&self) {
        self.is_running.store(true, Ordering::SeqCst);
    }

    pub fn init_sender(&mut self) {
        if self.socket.is_none() {
            return;
        }

        if self.job_action_channel_tx.is_none() || self.socket_send_channel_rx.is_none() {
            panic!("Channels not created");
        }

        let socket = self.socket.as_ref().unwrap();
        // Receiver will be moved to SocketSender thread.
        let socket_send_channel_rx = self.socket_send_channel_rx.take().unwrap();

        // other stuff to thread.
        let socket = Arc::clone(&socket);
        let time_to_die = Arc::clone(&self.time_to_die);
        let error_state_current = Arc::clone(&self.error_state_current);
        let error_state_previous = Arc::clone(&self.error_state_previous);
        let job_channel_tx = self.job_action_channel_tx.as_ref().unwrap().clone();

        // Thread creation with name.
        let sender_thread = thread::Builder::new().name(format!("Client sender thread"));
        match sender_thread.spawn(move || {
            (SocketSender::new(
                socket,
                socket_send_channel_rx,
                job_channel_tx,
                time_to_die,
                error_state_current,
                error_state_previous,
            ))
            .init()
        }) {
            Ok(handle) => self.push_handles(vec![handle]),
            Err(e) => {
                println!("{}", e);
            }
        }
    }

    pub fn init_job_channel_consumer(&mut self) {
        if self.job_action_channel_rx.is_none() {
            panic!("Channels not created");
        }

        let time_to_die = Arc::clone(&self.time_to_die);
        let jobs = Arc::clone(&self.jobs);

        // Receiver will be moved to Job Channel Consumer thread.
        let job_action_channel_rx = self.job_action_channel_rx.take().unwrap();

        // Action handler to add / remove jobs. Uses job_channel to communicate
        // with other threads.
        let job_thread = thread::Builder::new().name(format!("Job action consumer thread"));
        let handle_maybe = job_thread.spawn(move || {
            jobworkers::run_job_channel_consumer(jobs, job_action_channel_rx, time_to_die)
        });
        match handle_maybe {
            Ok(handle) => self.push_handles(vec![handle]),
            Err(e) => {
                panic!("{}", e);
            }
        };
    }

    // Job handler checks if there is problematic jobs
    pub fn init_job_handler(&mut self) {
        if self.socket_send_channel_tx.is_none() {
            println!("Socket channel not found");
            self.error_state_current.store(true, Ordering::SeqCst);
            return;
        }

        // let's take clone only of jobs hadhmap inside jobs struct.
        let jobs = Arc::clone(&self.jobs);
        let time_to_die = Arc::clone(&self.time_to_die);
        let job_channel_tx = self.socket_send_channel_tx.as_ref().unwrap().clone();

        // uses socket send channel to send re request, when job is not received in given time.
        // worker to handle packages
        let job_thread = thread::Builder::new().name(format!("Job worker thread"));
        let handle_maybe = job_thread
            .spawn(move || jobworkers::run_job_handler(jobs, job_channel_tx, time_to_die));
        match handle_maybe {
            Ok(handle) => {
                self.push_handles(vec![handle]);
            }
            Err(e) => {
                panic!("Thread is not running ... {}", e);
            }
        }
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
        match &self.socket_send_channel_tx {
            None => {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Socket send channel has not been initialized.",
                ));
            }
            Some(tx) => {
                // Create a job to follow up server response.
                // Job thread will ask to send new request if job fails.
                let next_job_handle = self.jobs.get_next_job_handle();
                let job_should_complite = (self.jobs.get_job_finish_time_average() * 5.0) as u128;
                let job_type: JobType = (ServerJob::NoServerAction, client_job_type);

                // let's add index and job type to a job.
                let job = Job::new(next_job_handle, job_type, raw_data, job_should_complite);
                // data has now index and job_type as 2 first bytes, rest is actual raw_data.
                let data = job.get_raw_data();

                // Job is inserted to Jobs via job_action_channel
                // (*jobs_changer).jobs.insert(next_job_handle, job);
                let job_action_channel_tx = self.job_action_channel_tx.as_ref().unwrap();
                match job_action_channel_tx.send((
                    JobAction::ADD,
                    next_job_handle,
                    Some(job.clone()),
                )) {
                    Err(e) => {
                        println!("Job action channel hang up: {}", e);
                    }
                    Ok(()) => {}
                }

                let result = tx.send((data, job));
                if result.is_ok() {
                    return Ok(());
                }
                return Err(Error::new(ErrorKind::Other, "Send channel hang up."));
            }
        }
    }
}
