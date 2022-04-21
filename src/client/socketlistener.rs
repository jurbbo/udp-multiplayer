use crate::client::Arc;
use crate::client::Mutex;
use crate::client::RequestEvents;
use crate::client::UdpSocket;
use crate::protocol::datastructure::get_u8_from_bit_slice;
use crate::requests::jobs::Jobs;
use crate::requests::jobtype::get_job_type;
use crate::requests::jobtype::JobType;
use crate::requests::jobtype::ServerJob;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

pub struct SocketListener<S: 'static> {
    jobs: Arc<Mutex<Jobs>>,
    time_to_die: Arc<AtomicBool>,
    socket: Arc<UdpSocket>,
    events: Arc<Mutex<S>>,
    error_state_current: Arc<AtomicBool>,
    error_state_previous: Arc<AtomicBool>,
}

impl<S: RequestEvents + Send + Sync> SocketListener<S> {
    pub fn new(
        jobs: &Arc<Mutex<Jobs>>,
        socket: &Arc<UdpSocket>,
        time_to_die: &Arc<AtomicBool>,
        events: &Arc<Mutex<S>>,
        error_state_current: &Arc<AtomicBool>,
        error_state_previous: &Arc<AtomicBool>,
    ) -> SocketListener<S>
    where
        S: RequestEvents + Send + Sync,
    {
        SocketListener {
            jobs: Arc::clone(jobs),
            socket: Arc::clone(socket),
            time_to_die: Arc::clone(time_to_die),
            events: Arc::clone(events),
            error_state_current: Arc::clone(error_state_current),
            error_state_previous: Arc::clone(error_state_previous),
        }
    }

    pub fn init_listener(&mut self) {
        loop {
            // break loop if server is closing...
            if self.time_to_die.load(Ordering::SeqCst) {
                break;
            }

            let raw_data_maybe = self.read_socket();

            // if conenction state has changed, raise event.
            if self.error_state_current.load(Ordering::SeqCst)
                != self.error_state_previous.load(Ordering::SeqCst)
            {
                self.error_state_previous.store(
                    self.error_state_current.load(Ordering::SeqCst),
                    Ordering::SeqCst,
                );
                let mut events_changer = self.events.lock().unwrap();
                (*events_changer)
                    .on_connection_state_change(self.error_state_current.load(Ordering::SeqCst));
            }

            // Thread sleeps 1 second, if connection is lost...
            if raw_data_maybe.is_none() {
                thread::sleep(Duration::new(1, 0));
                continue;
            }
            // Data is received, lets work on it.
            let raw_data = raw_data_maybe.unwrap();

            let index_and_type_maybe = self.get_index_and_type(&raw_data);

            // Received misformed data
            if index_and_type_maybe.is_none() {
                continue;
            }

            let (job_index, job_type) = index_and_type_maybe.unwrap();
            self.create_request_event(job_index, job_type.0, raw_data);
        }
    }

    // Read socket, blocking function, return None is socket read fails for connection error.
    fn read_socket(&self) -> Option<Vec<u8>> {
        let mut buf = [0; 10];

        let result = self.socket.recv_from(&mut buf);
        if result.is_err() {
            self.error_state_current.store(true, Ordering::SeqCst);
            return None;
        }

        if self.error_state_current.load(Ordering::SeqCst) {
            self.error_state_current.store(false, Ordering::SeqCst);
        }

        let (number_of_bytes, _src_addr) = result.unwrap();
        let raw_data = &mut buf[..number_of_bytes];
        Some(raw_data.to_vec())
    }

    fn get_index_and_type(&self, raw_data: &Vec<u8>) -> Option<(u8, JobType)> {
        if raw_data.len() < 2 {
            let mut jobs_changer = self.jobs.lock().unwrap();
            (*jobs_changer).packages_failed += 1;
            return None;
        }
        let index = raw_data[0];
        let job_type = tranfer_job_type_from_number(&raw_data[1]);
        if job_type.is_none() {
            let mut jobs_changer = self.jobs.lock().unwrap();
            (*jobs_changer).packages_failed += 1;
            return None;
        }
        Some((index, job_type.unwrap()))
    }
    fn handle_data(&self, job_index: u8, job_type: JobType, raw_data: Vec<u8>)
    where
        S: RequestEvents + Send + Sync,
    {
        let mut job_duration = Duration::new(0, 0);

        // Job handling for operations fired from client
        match &job_type {
            JobType::DATA_PUSH_RECEIVED => { /* no job handling for server push operations */ }
            __ => {
                let mut jobs_changer = self.jobs.lock().unwrap();
                let job_maybe = (*jobs_changer).jobs.get_mut(&job_index);
                if job_maybe.is_none() {
                    (*jobs_changer).packages_failed += 1;
                    let mut events_changer = self.events.lock().unwrap();
                    (*events_changer).on_error();
                    return;
                }
                let job = job_maybe.unwrap();
                job_duration = job.finish();
                (*jobs_changer).add_finish_time(job_duration);
                (*jobs_changer).packages_handled += 1;
            }
        }

        match &job_type {
            JobType::DATA_PUSH_ACTION => {
                let mut events_changer = self.events.lock().unwrap();
                (*events_changer).on_data_push_action(raw_data);
            }
            JobType::DATA_PUSH_RECEIVED => {
                println!("received");
                if raw_data.len() < 3 {
                    let mut jobs_changer = self.jobs.lock().unwrap();
                    (*jobs_changer).packages_failed += 1;
                    let mut events_changer = self.events.lock().unwrap();
                    (*events_changer).on_error();
                    return;
                }
                let player = raw_data[2];
                let mut events_changer = self.events.lock().unwrap();
                (*events_changer).on_data_push_received(player, raw_data);
            }
            JobType::DATA_REQUEST => {
                println!("request");
                if raw_data.len() < 4 {
                    let mut jobs_changer = self.jobs.lock().unwrap();
                    (*jobs_changer).packages_failed += 1;
                    let mut events_changer = self.events.lock().unwrap();
                    (*events_changer).on_error();
                    return;
                }
                let player = raw_data[2];
                let mut events_changer = self.events.lock().unwrap();
                (*events_changer).on_data_push_received(player, raw_data);
            }
            JobType::PLAYER_ENTER => {
                println!("enter");
                if raw_data.len() < 4 {
                    let mut jobs_changer = self.jobs.lock().unwrap();
                    (*jobs_changer).packages_failed += 1;
                    let mut events_changer = self.events.lock().unwrap();
                    (*events_changer).on_error();
                    return;
                }
                let player_number = raw_data[2];
                let mut events_changer = self.events.lock().unwrap();
                (*events_changer).on_player_enter(player_number, raw_data);
            }
            JobType::PLAYER_LEAVE => {
                println!("leave");
                let mut events_changer = self.events.lock().unwrap();
                (*events_changer).on_player_leave(raw_data);
            }
            JobType::PING => {
                println!("ping");

                let mut events_changer = self.events.lock().unwrap();
                (*events_changer).on_pong(job_duration);
            }
        }
    }
}
