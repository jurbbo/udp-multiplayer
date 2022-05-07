use crate::requests::jobs::Jobs;
use crate::server::connection::Connections;
use crate::server::server::time::Instant;
use std::io::Error;
use std::io::ErrorKind;
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;
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
    handles: Option<Vec<JoinHandle<()>>>,
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
            handles: None,
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
                "{} / conenctions in 10 sec {}, bytes received {}, bytes sent {}, player number {}",
                ip,
                connection.connections_count,
                connection.bytes_received,
                connection.bytes_send,
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
            let handle = thread::spawn(move || loop {
                // break if server is closing...
                if time_to_die.load(Ordering::SeqCst) {
                    break;
                }

                let mut buf = [0; 10];
                let (number_of_bytes, src_addr) =
                    socket.recv_from(&mut buf).expect("Didn't receive data");
                let filled_buf = &mut buf[..number_of_bytes];
                // return message to client
                socket.send_to(filled_buf, src_addr).expect("Socket fail!");

                let handle = filled_buf[0];
                let data_type = filled_buf[1];

                // lock connections for changes.
                let mut connections_changer = connections.lock().unwrap();

                // add connection stats to to connection struct
                if (*connections_changer).connections.contains_key(&src_addr) {
                    let mut connection = (*connections_changer)
                        .connections
                        .get_mut(&src_addr)
                        .unwrap();
                    connection.connections_count += 1;
                    connection.bytes_received += number_of_bytes as i128;
                } else {
                    let result = (*connections_changer)
                        .create_new_connection(src_addr, "Tyypin nimi".to_string());
                    if !result {
                        println!("Failed to add new connection...");
                    }
                }

                // send data (share data) to other connections
                if data_type == 1 {
                    let mut player_number = 0;
                    // get sender player number
                    if (*connections_changer).connections.contains_key(&src_addr) {
                        player_number = (*connections_changer)
                            .connections
                            .get(&src_addr)
                            .unwrap()
                            .player_number;
                    }

                    for (ip, connection) in &mut (*connections_changer).connections {
                        if ip != &src_addr {
                            let raw_data = [handle, 2 as u8, player_number];
                            socket.send_to(&raw_data, ip).expect("Socket fail!");
                            connection.bytes_send += filled_buf.len() as i128;
                        }
                    }
                }
            });
            handles.push(handle);
        }
        self.push_handles(handles);
        Ok(())
    }
}
