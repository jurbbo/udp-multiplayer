use crate::protocol::bithelpers::get_u8_from_bit_slice;
use crate::protocol::datahelpers::{create_player_created_response, create_player_enter_push};
use crate::protocol::Protocol;
use crate::requests::jobs::Jobs;
use crate::requests::jobtype::get_job_single_byte;
use crate::requests::jobtype::get_job_type;
use crate::requests::jobtype::ClientJob;
use crate::requests::jobtype::JobType;
use crate::requests::jobtype::ServerJob;
use crate::server::connection::Connections;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::string::String;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::thread;
use std::time::Duration;

pub struct ServerSocketListener {
    connections: Arc<Mutex<Connections>>,
    protocols: Arc<Protocol>,
    jobs: Arc<Mutex<Jobs>>,
    time_to_die: Arc<AtomicBool>,
    socket: Arc<UdpSocket>,
    error_state_current: Arc<AtomicBool>,
    error_state_previous: Arc<AtomicBool>,
}

impl ServerSocketListener {
    pub fn new(
        connections: Arc<Mutex<Connections>>,
        jobs: Arc<Mutex<Jobs>>,
        protocols: Arc<Protocol>,
        socket: Arc<UdpSocket>,
        time_to_die: Arc<AtomicBool>,
        error_state_current: Arc<AtomicBool>,
        error_state_previous: Arc<AtomicBool>,
    ) -> ServerSocketListener {
        ServerSocketListener {
            jobs: jobs,
            protocols: protocols,
            socket: socket,
            time_to_die: time_to_die,
            error_state_current: error_state_current,
            error_state_previous: error_state_previous,
            connections: connections,
        }
    }

    pub fn run(&mut self) {
        loop {
            // break loop if server is closing...
            if self.time_to_die.load(Ordering::SeqCst) {
                break;
            }

            let raw_data_maybe = self.read_socket();

            // if connection state has changed, raise event.
            if self.error_state_current.load(Ordering::SeqCst)
                != self.error_state_previous.load(Ordering::SeqCst)
            {
                self.error_state_previous.store(
                    self.error_state_current.load(Ordering::SeqCst),
                    Ordering::SeqCst,
                );

                self.handle_connection_change(self.error_state_current.load(Ordering::SeqCst));

                //let mut events_changer = self.events.lock().unwrap();
                //(*events_changer)
                //    .on_connection_state_change(self.error_state_current.load(Ordering::SeqCst));
            }

            // Thread sleeps 1 second, if connection is lost...
            if raw_data_maybe.is_none() {
                thread::sleep(Duration::new(1, 0));
                continue;
            }

            // Data is received, lets work on it.
            let (raw_data, src_addr) = raw_data_maybe.unwrap();

            self.set_connection_stats(src_addr, raw_data.len() as i128);

            let index_and_type_maybe = self.get_index_and_type(&raw_data);

            // Received misformed data
            if index_and_type_maybe.is_none() {
                continue;
            }

            let (job_index, job_type) = index_and_type_maybe.unwrap();
            // job type contains serverjob and clientjob as tuple, second item is client request job.
            self.handle_data(src_addr, job_index, job_type, raw_data);
        }
    }

    // Read socket, blocking method, return None is socket read fails for connection error.
    fn read_socket(&self) -> Option<(Vec<u8>, SocketAddr)> {
        let mut buf = [0; 100];

        let result = self.socket.recv_from(&mut buf);
        if result.is_err() {
            self.error_state_current.store(true, Ordering::SeqCst);
            return None;
        }

        if self.error_state_current.load(Ordering::SeqCst) {
            self.error_state_current.store(false, Ordering::SeqCst);
        }

        let (number_of_bytes, src_addr) = result.unwrap();
        let raw_data = &mut buf[..number_of_bytes];
        Some((raw_data.to_vec(), src_addr))
    }

    fn send_to_socket(
        &self,
        src_addr: SocketAddr,
        raw_data: &[u8],
        connections_changer: &mut MutexGuard<Connections>,
    ) {
        let result = self.socket.send_to(&raw_data, src_addr);

        if result.is_err() {
            self.error_state_current.store(true, Ordering::SeqCst);
            return;
        }

        if self.error_state_current.load(Ordering::SeqCst) {
            self.error_state_current.store(false, Ordering::SeqCst);
        }

        let connection_maybe = (*connections_changer).connections.get_mut(&src_addr);
        if connection_maybe.is_some() {
            let mut connection = connection_maybe.unwrap();
            (*connection).bytes_send += raw_data.len() as i128;
        }
    }

    fn get_index_and_type(&self, raw_data: &Vec<u8>) -> Option<(u8, JobType)> {
        if raw_data.len() < 2 {
            let mut jobs_changer = self.jobs.lock().unwrap();
            (*jobs_changer).packages_failed += 1;
            return None;
        }
        let index = raw_data[0];

        let server_client = (
            get_u8_from_bit_slice(raw_data[1], 0, 4),
            get_u8_from_bit_slice(raw_data[1], 4, 4),
        );

        let job_type = get_job_type(&server_client);
        if job_type.is_none() {
            let mut jobs_changer = self.jobs.lock().unwrap();
            (*jobs_changer).packages_failed += 1;
            return None;
        }
        Some((index, job_type.unwrap()))
    }

    fn handle_connection_change(&self, error_state: bool) {
        print!("hello {}", error_state);
    }

    fn set_connection_stats(&self, src_addr: SocketAddr, number_of_bytes: i128) {
        // lock connections for changes.
        let mut connections_changer = self.connections.lock().unwrap();

        // add connection stats to to connection struct
        if (*connections_changer).connections.contains_key(&src_addr) {
            let mut connection = (*connections_changer)
                .connections
                .get_mut(&src_addr)
                .unwrap();
            connection.connections_count += 1;
            connection.bytes_received += number_of_bytes;
        }
    }

    fn fail_package(&self) {
        let mut jobs_changer = self.jobs.lock().unwrap();
        (*jobs_changer).packages_failed += 1;
    }

    // Call implemented trait (RequestEvent) methods according
    // what kind of (JobType) data is reveiced.
    fn handle_data(
        &mut self,
        src_addr: SocketAddr,
        job_index: u8,
        job: JobType,
        raw_data: Vec<u8>,
    ) {
        let client_request_type = job.1;

        let mut job_duration = Duration::new(0, 0);

        // lock connections for changes.
        // Job handling for operations fired from client
        /*
        match &client_request_type {
            ClientJob::DataPush => { /* no job handling for server push operations */ }
            ClientJob::PlayerEnterPush => { /* no job handling */ }
            ClientJob::PlayerLeavePush => {}
            __ => {
                let mut jobs_changer = self.jobs.lock().unwrap();
                let job_maybe = (*jobs_changer).jobs.get_mut(&job_index);
                if job_maybe.is_none() {
                    (*jobs_changer).packages_failed += 1;
                    //let mut events_changer = self.events.lock().unwrap();
                    //(*events_changer).on_error();
                    return;
                }
                let job = job_maybe.unwrap();
                job_duration = job.finish();
                (*jobs_changer).add_finish_time(job_duration);
                (*jobs_changer).packages_handled += 1;
            }
        }*/

        let mut connections_changer = self.connections.lock().unwrap();

        match &client_request_type {
            ClientJob::NoClientAction => {}

            ClientJob::DataPushRequest => {
                let job: JobType = (ServerJob::DataPush, client_request_type.clone());
                let job_single_byte = get_job_single_byte(&job);

                let mut player_number = 0;
                // get sender player number
                if (*connections_changer).connections.contains_key(&src_addr) {
                    player_number = (*connections_changer)
                        .connections
                        .get(&src_addr)
                        .unwrap()
                        .player_number;
                }

                // Not a registered player.
                if player_number == 0 {
                    return;
                }

                // create return data and job single byte for DataPush.
                let mut dynamic_data = raw_data[2..].to_vec();

                let data_string = String::from_utf8_lossy(&dynamic_data);
                println!("data {}", data_string);
                let mut data = vec![job_index, job_single_byte, player_number];
                data.append(&mut dynamic_data);

                // Send data to everyone but the request sender.
                let mut connection_addresses: Vec<SocketAddr> = Vec::new();
                for (connection_addr, _connection) in &connections_changer.connections {
                    if *connection_addr != src_addr {
                        connection_addresses.push(*connection_addr);
                    }
                }
                for addr in connection_addresses {
                    self.send_to_socket(addr, &data, &mut connections_changer);
                }

                /*
                for (ip, connection) in &mut (*connections_changer).connections {
                    if ip != &src_addr {
                        let data = [job_index, job_single_byte, player_number];
                        //let _result = self.socket.send_to(&data, ip);
                        connection.bytes_send += raw_data.len() as i128;
                    }
                }*/

                // Inform client that data push has been done.
                let job: JobType = (ServerJob::DataPushDoneResponse, client_request_type);
                let job_single_byte = get_job_single_byte(&job);
                let data = [job_index, job_single_byte];
                self.send_to_socket(src_addr, &data, &mut connections_changer);

                //self.socket.send_to(&data, src_addr).expect("Socket fail!");
            }
            ClientJob::DataRequest => {
                /*
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
                */
            }
            ClientJob::PlayerEnterRequest => {
                println!("Player enter request");

                let raw_len = raw_data.len();

                if raw_len < 4 {
                    println!("package too short.");

                    self.fail_package();
                    let job: JobType = (ServerJob::PlayerCreatedResponse, client_request_type);
                    let job_single_byte = get_job_single_byte(&job);
                    let data = [job_index, job_single_byte, 101];
                    self.send_to_socket(src_addr, &data, &mut connections_changer);
                    return;
                }

                let player_name = String::from_utf8_lossy(&raw_data[2..raw_len]);

                let is_name_taken = (*connections_changer).is_name_taken(player_name.to_string());
                if is_name_taken {
                    println!("name is taken");

                    self.fail_package();
                    let job: JobType = (ServerJob::PlayerCreatedResponse, client_request_type);
                    let job_single_byte = get_job_single_byte(&job);
                    let data = [job_index, job_single_byte, 100];
                    self.send_to_socket(src_addr, &data, &mut connections_changer);
                    return;
                }
                let player_number =
                    (*connections_changer).create_new_connection(src_addr, player_name.to_string());
                (src_addr, player_name.to_string());
                println!("{}", player_number.unwrap());
                if player_number.is_none() {
                    println!("too many players");

                    let job: JobType = (ServerJob::PlayerCreatedResponse, client_request_type);
                    let job_single_byte = get_job_single_byte(&job);
                    let data = [job_index, job_single_byte, 102];
                    self.send_to_socket(src_addr, &data, &mut connections_changer);
                    return;
                }

                let job: JobType = (
                    ServerJob::PlayerCreatedResponse,
                    client_request_type.clone(),
                );
                let job_single_byte = get_job_single_byte(&job);
                //let data = [job_index, job_single_byte, 1, player_number.unwrap()];

                // let's create player created response. With array data
                // of other players, names, numbers, ips.
                let raw_data_result = create_player_created_response(
                    &self.protocols,
                    1,
                    player_name.to_string(),
                    player_number.unwrap(),
                    &connections_changer.connections,
                );

                if raw_data_result.is_ok() {
                    let mut data = vec![job_index, job_single_byte];
                    data.append(&mut raw_data_result.unwrap());
                    self.send_to_socket(src_addr, &data, &mut connections_changer);
                    // logging...
                    println!(
                        "'{}' entered as player number {}",
                        player_name,
                        player_number.unwrap()
                    );
                } else {
                    // Something wrong with response builder.
                    println!(
                        "Failed to create player created response. Error: {}",
                        raw_data_result.unwrap_err()
                    );

                    self.fail_package();
                    let job: JobType = (ServerJob::PlayerCreatedResponse, client_request_type);
                    let job_single_byte = get_job_single_byte(&job);
                    let data = [job_index, job_single_byte, 100];
                    self.send_to_socket(src_addr, &data, &mut connections_changer);
                    return;
                }

                let job: JobType = (ServerJob::PlayerEnterPush, client_request_type);
                let job_single_byte = get_job_single_byte(&job);

                // Send new player information to other players.
                let mut connection_addresses: Vec<SocketAddr> = Vec::new();
                for (connection_addr, _connection) in &connections_changer.connections {
                    if *connection_addr != src_addr {
                        connection_addresses.push(*connection_addr);
                    }
                }
                for addr in connection_addresses {
                    let raw_data_result = create_player_enter_push(
                        &self.protocols,
                        player_name.to_string(),
                        player_number.unwrap(),
                        src_addr,
                    );

                    if raw_data_result.is_ok() {
                        let mut data = vec![job_index, job_single_byte];
                        data.append(&mut raw_data_result.unwrap());

                        self.send_to_socket(addr, &data, &mut connections_changer);
                    }
                }
                /*
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
                */
            }
            ClientJob::PlayerLeaveRequest => {
                /*
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
                */
            }
            ClientJob::PingRequest => {
                // Inform client that data push has been done.
                let job: JobType = (ServerJob::PongResponse, client_request_type);
                let job_single_byte = get_job_single_byte(&job);
                let data = [job_index, job_single_byte];
                self.socket.send_to(&data, src_addr).expect("Socket fail!");
            }
        }
    }
}
