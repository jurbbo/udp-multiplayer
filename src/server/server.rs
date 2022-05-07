use crate::requests::jobs::Jobs;
use crate::server::connection::Connections;
use crate::server::socketlistener::ServerSocketListener;
use std::io::Error;
use std::io::ErrorKind;
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Instant;
use std::{thread, time};

pub struct Server {
    jobs: Arc<Mutex<Jobs>>,
    time_to_die: Arc<AtomicBool>,
    error_state_previous: Arc<AtomicBool>,
    error_state_current: Arc<AtomicBool>,
    error_state_start_time: Option<Instant>,
    thread_handles: Option<Vec<JoinHandle<()>>>,
    socket: Option<Arc<UdpSocket>>,
    threads_count: u8,
    connections: Arc<Mutex<Connections>>,
    //handle_data_cb: Arc<Mutex<fn(job_type: JobType, raw_data: &mut [u8])>>,
}

impl Server {
    pub fn new(
        threads_count: u8,
        //handle_data_cb: fn(job_type: JobType, raw_data: &mut [u8]),
    ) -> Server {
        Server {
            jobs: Arc::new(Mutex::new(Jobs::new())),
            time_to_die: Arc::new(AtomicBool::new(false)),
            error_state_previous: Arc::new(AtomicBool::new(false)),
            error_state_current: Arc::new(AtomicBool::new(false)),
            error_state_start_time: None,
            thread_handles: None,
            socket: None,
            threads_count: threads_count,
            //handle_data_cb: Arc::new(Mutex::new(handle_data_cb)),
            connections: Arc::new(Mutex::new(Connections::new())),
        }
    }

    // Run until Ctrl-C in pressed. This is blocking!
    pub fn run(&mut self) {
        let result = self.init_listeners();
        self.init_status();

        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();
        ctrlc::set_handler(move || {
            r.store(false, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");
        while running.load(Ordering::SeqCst) {}
        println!("");
        println!("Graceful shutdown...");
        self.die();
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

    pub fn connect(&mut self, local_ip: String) -> Result<(), std::io::Error> {
        let socket_result = UdpSocket::bind(local_ip);
        if socket_result.is_err() {
            let err = socket_result.unwrap_err();
            return Err(err);
        }

        let socket = socket_result.unwrap();

        self.socket = Some(Arc::new(socket));

        Ok(())
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

    pub fn init_status(&mut self) {
        let pause_time = time::Duration::from_millis(100);
        let connections = Arc::clone(&self.connections);
        let time_to_die = Arc::clone(&self.time_to_die);

        let handle = thread::spawn(move || loop {
            for _count in 1..100 {
                thread::sleep(pause_time);
                if time_to_die.load(Ordering::SeqCst) {
                    break;
                }
            }

            if time_to_die.load(Ordering::SeqCst) {
                break;
            }

            println!("List of connections in 10 seconds: ");
            let mut connection_changer = connections.lock().unwrap();
            for (ip, connection) in &mut (*connection_changer).connections {
                println!(
                "{} / connections in 10 sec {}, bytes received {}, bytes sent {}, player name {}, player number {}",
                ip,
                connection.connections_count,
                connection.bytes_received,
                connection.bytes_send,
                connection.player_name,
                connection.player_number
            );
                (*connection).connections_count = 0;
            }
        });

        self.push_handles(vec![handle]);
    }

    pub fn init_listeners(&mut self) -> Result<(), std::io::Error> {
        if self.socket.is_none() {
            return Err(Error::new(
                ErrorKind::Other,
                "Cannot send without activated socket. Hint: has port or ip address failed?",
            ));
        }
        let socket = self.socket.as_ref().unwrap();

        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        for _handle_index in 1..self.threads_count {
            let socket = Arc::clone(socket);
            let connections = Arc::clone(&self.connections);
            let time_to_die = Arc::clone(&self.time_to_die);
            let error_state_current = Arc::clone(&self.error_state_current);
            let error_state_previous = Arc::clone(&self.error_state_previous);
            let jobs = Arc::clone(&self.jobs);

            // worker for listening server data starts here
            let handle = thread::spawn(move || {
                (ServerSocketListener::new(
                    connections,
                    jobs,
                    socket,
                    time_to_die,
                    error_state_current,
                    error_state_previous,
                ))
                .run()
            });

            handles.push(handle);
        }
        self.push_handles(handles);
        Ok(())
    }
}
