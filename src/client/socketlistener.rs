use crate::client::Arc;
use crate::client::Mutex;
use crate::client::RequestEvents;
use crate::client::UdpSocket;
use crate::requests::jobs::Jobs;
use crate::requests::jobtype::{tranfer_job_type_from_number, JobType};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

pub struct SocketListener<S: 'static> {
    jobs: Arc<Mutex<Jobs>>,
    time_to_die: Arc<AtomicBool>,
    socket: Arc<UdpSocket>,
    events: Arc<Mutex<S>>,
}

impl<S: RequestEvents + std::marker::Send + std::marker::Sync> SocketListener<S> {
    pub fn new(
        jobs: &Arc<Mutex<Jobs>>,
        socket: &Arc<UdpSocket>,
        time_to_die: &Arc<AtomicBool>,
        events: &Arc<Mutex<S>>,
    ) -> SocketListener<S>
    where
        S: RequestEvents + Send + Sync,
    {
        SocketListener {
            jobs: Arc::clone(jobs),
            socket: Arc::clone(socket),
            time_to_die: Arc::clone(time_to_die),
            events: Arc::clone(events),
        }
    }

    pub fn init_listener(&mut self) {
        loop {
            // break loop if server is closing...
            if self.time_to_die.load(Ordering::SeqCst) {
                break;
            }

            let raw_data = self.read_socket();

            let index_and_type_maybe = self.get_index_and_type(&raw_data);

            // Received misformed data
            if index_and_type_maybe.is_none() {
                continue;
            }

            let (job_index, job_type) = index_and_type_maybe.unwrap();
            // data is received, lets work on it.

            self.handle_data(job_index, job_type, raw_data);
        }
    }
    fn read_socket(&self) -> Vec<u8> {
        let mut buf = [0; 10];
        let (number_of_bytes, _src_addr) = self
            .socket
            .recv_from(&mut buf)
            .expect("Socket failed on receive");
        let raw_data = &mut buf[..number_of_bytes];
        raw_data.to_vec()
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
