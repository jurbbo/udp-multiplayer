use crate::client::Arc;
use crate::client::UdpSocket;
use crate::requests::Job;
use crate::requests::JobAction;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

pub struct SocketSender {
    socket: Arc<UdpSocket>,
    send_channel_rx: Receiver<(Vec<u8>, Job)>,
    job_channel_tx: Sender<(JobAction, u8, Option<Job>)>,
    error_state_current: Arc<AtomicBool>,
    error_state_previous: Arc<AtomicBool>,
    time_to_die: Arc<AtomicBool>,
}

impl SocketSender {
    pub fn new(
        socket: Arc<UdpSocket>,
        send_channel_rx: Receiver<(Vec<u8>, Job)>,
        job_channel_tx: Sender<(JobAction, u8, Option<Job>)>,
        time_to_die: Arc<AtomicBool>,
        error_state_current: Arc<AtomicBool>,
        error_state_previous: Arc<AtomicBool>,
    ) -> SocketSender {
        SocketSender {
            time_to_die: time_to_die,
            socket: socket,
            send_channel_rx: send_channel_rx,
            job_channel_tx: job_channel_tx,
            error_state_current: error_state_current,
            error_state_previous: error_state_previous,
        }
    }

    fn read_raw_data_from_send_channel(&self) -> Option<(Vec<u8>, Job)> {
        let result = self.send_channel_rx.recv();
        if result.is_err() {
            println!("Send channel reveicer hang up.");
            return None;
        }
        Some(result.unwrap())
    }

    fn error_state_event_raiser(&self, _error_message: Option<String>) {
        // if conenction state has changed, raise event.
        if self.error_state_current.load(Ordering::SeqCst)
            != self.error_state_previous.load(Ordering::SeqCst)
        {
            self.error_state_previous.store(
                self.error_state_current.load(Ordering::SeqCst),
                Ordering::SeqCst,
            );
            // TODO:: HANDLE ERROR STATE CHANGE.
            /*
            let mut events_changer = self.events.lock().unwrap();
            (*events_changer)
                .on_connection_state_change(self.error_state_current.load(Ordering::Relaxed));
            */
            println!(
                "Error state is {}",
                self.error_state_current.load(Ordering::Relaxed)
            );
        }
    }

    pub fn init(&mut self) {
        loop {
            // break loop if server is closing...
            if self.time_to_die.load(Ordering::Relaxed) {
                break;
            }

            let raw_data_maybe = self.read_raw_data_from_send_channel();

            // Thread sleeps 100 millis, if sender channel producers hang up...
            // Something must be fucked, if this happens.
            if raw_data_maybe.is_none() {
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            let (raw_data, job) = raw_data_maybe.unwrap();

            let result = self.socket.send(&raw_data);
            match result {
                Err(e) => {
                    // send fails ... JOB REMOVAL from jobs list
                    self.job_channel_tx
                        .send((JobAction::REMOVE, job.get_handle(), None));

                    self.error_state_previous.store(true, Ordering::SeqCst);
                    self.error_state_event_raiser(Some(e.to_string()));
                }
                Ok(send_buffer_size) => {
                    if self.error_state_current.load(Ordering::SeqCst) == true {
                        self.error_state_previous.store(false, Ordering::SeqCst);
                    }
                    self.error_state_event_raiser(None);
                }
            }
        }
    }
}
