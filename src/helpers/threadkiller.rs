use crate::requests::{ClientJob, Job, JobAction, JobType, ServerJob};
use crate::socket::SocketCombatible;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time;

// Let's send empty requests to server or client, so that blocking socket listener
// will stop blocking, and server / client instance can close.
pub fn thread_killer<T: SocketCombatible>(closing_instance: &T) -> Option<JoinHandle<()>> {
    let mut ip_string: String = "".to_string();
    if closing_instance.get_ip().is_none() || closing_instance.get_port().is_none() {
        return None;
    }
    if let IpAddr::V4(ipv4) = closing_instance.get_ip().unwrap() {
        ip_string = format!(
            "{}.{}.{}.{}",
            ipv4.octets()[0],
            ipv4.octets()[1],
            ipv4.octets()[2],
            ipv4.octets()[3]
        );
    }

    let mut thread_killer_handle: Option<JoinHandle<()>> = None;

    for port in 49152..65535 {
        match UdpSocket::bind(format!("{}:{}", ip_string.to_string(), port)) {
            Err(_socket) => {}
            Ok(socket) => {
                let socket_arc = Arc::new(socket);
                let local_ip = Arc::new(closing_instance.get_ip().unwrap());
                let local_port = Arc::new(closing_instance.get_port().unwrap());
                let is_running = Arc::clone(&closing_instance.get_is_running_atomic());

                let thread_killer_thread =
                    thread::Builder::new().name(format!("Thread killer thread"));
                thread_killer_handle = Some(
                    thread_killer_thread
                        .spawn(move || {
                            let closing_instance_addr = SocketAddr::new(*local_ip, *local_port);
                            loop {
                                thread::sleep(time::Duration::from_millis(1000));
                                println!("{}", local_port);

                                let result =
                                    socket_arc.send_to(&[0, 0, 0, 0, 0], closing_instance_addr);
                                match result {
                                    Err(_e) => break,
                                    Ok(_data) => {}
                                }

                                if is_running.load(Ordering::SeqCst) == false {
                                    break;
                                }
                            }
                        })
                        .unwrap(),
                );
                break;
            }
        }
    }

    return thread_killer_handle;
}

pub fn client_channel_killer(
    job_action_channel_tx: Sender<(JobAction, u8, Option<Job>)>,
    socket_send_channel_tx: Sender<(Vec<u8>, Job)>,
    is_running: Arc<AtomicBool>,
) -> Option<JoinHandle<()>> {
    let thread_killer_thread = thread::Builder::new().name(format!("Thread killer thread"));
    let result = thread_killer_thread.spawn(move || {
        let mut is_socket_send_channel_dead = false;
        let mut is_job_action_channel_dead = false;
        loop {
            thread::sleep(time::Duration::from_millis(50));
            println!("pöö");

            if !is_job_action_channel_dead {
                is_job_action_channel_dead = kill_job_action_channel(&job_action_channel_tx);
            }

            if !is_socket_send_channel_dead {
                is_socket_send_channel_dead = kill_socket_send_channel(&socket_send_channel_tx);
            }

            if is_running.load(Ordering::SeqCst) == false {
                break;
            }
        }
    });

    match result {
        Ok(handle) => return Some(handle),
        Err(e) => {
            println!("{}", e);
            return None;
        }
    }
}

pub fn kill_job_action_channel(
    job_action_channel_tx: &Sender<(JobAction, u8, Option<Job>)>,
) -> bool {
    let result = job_action_channel_tx.send((JobAction::INCHANDLED, 0, None));
    if result.is_err() {
        return true;
    }
    false
}

pub fn kill_socket_send_channel(socket_send_channel_tx: &Sender<(Vec<u8>, Job)>) -> bool {
    let mut nonsense_data = [0, 0, 0, 0, 0].to_vec();
    let job_type: JobType = (ServerJob::NoServerAction, ClientJob::NoClientAction);
    let nonsense_job = Job::new(0, job_type, &mut nonsense_data, 10);
    let result = socket_send_channel_tx.send((nonsense_data, nonsense_job));
    if result.is_err() {
        return true;
    }
    false
}
