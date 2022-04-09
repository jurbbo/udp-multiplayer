use crate::client::Client;
use crate::client::RequestEvents;
use crate::requests::jobtype::JobType;
use std::io::stdin;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

/*
impl RequestEvents for Client {
    fn handle_data(job_type: u8, raw_data: std::vec::Vec<u8>) {
        println!("Data");
    }
    fn handle_ping(_: std::time::Instant) {
        todo!()
    }&mut self&mut self
}
*/

fn handle_data_cb(job_type: JobType, raw_data: &mut [u8]) {
    match job_type {
        JobType::DATA_PUSH_ACTION => {}
        JobType::DATA_PUSH_RECEIVED => {}
        JobType::DATA_REQUEST => {}
        JobType::PLAYER_ENTER => {}
        JobType::PLAYER_LEAVE => {}
        JobType::PING => {
            println!("Pong")
        }
    }
}

struct Eventful {
    pub count: u128,
}

impl RequestEvents for Eventful {
    fn handle_data(&mut self, request: JobType, raw_data: &mut [u8]) {
        println!(
            "data events {} {}.. count {}",
            raw_data[0], raw_data[1], self.count
        );
        self.count += 1;
    }

    fn handle_ping(&mut self, _: std::time::Instant) {
        todo!()
    }
}

pub fn testclient() {
    println!();
    println!("UDP Rusting client...");

    let first: u8 = 0b10101110;
    let second: u8 = 0b11110010;
    let third: u8 = 0b10001111;

    let bytes = vec![first, second, third];

    /*
    let mut data_s = DataStructure::new(bytes);

    data_s.add_structure(
        "test1".to_string(),
        4,
        2,
        DataType::NUMBERDATA,
        "Testing 1, count 2 bits".to_string(),
    );

    data_s.add_structure(
        "test2".to_string(),
        6,
        2,
        DataType::NUMBERDATA,
        "Testing 2, count 2 bits".to_string(),
    );

    println!(
        "{}",
        data_s.get_u8_from_bit_slice("test1".to_string()).unwrap()
    );
    println!(
        "{}",
        data_s.get_u8_from_bit_slice("test2".to_string()).unwrap()
    );
    */

    println!("port: ");
    let mut local_ip: String = "127.0.0.1:".to_owned();
    let mut port = String::new();
    stdin().read_line(&mut port).expect("Shit happened...");
    local_ip.push_str(&port.trim());

    let mut client = Client::new(&local_ip, "localhost:11111", 3, handle_data_cb);
    let events = Arc::new(Mutex::new(Eventful { count: 0 }));
    client.runn(events);

    println!("Pause between data grams: ");
    let mut pause_time = String::new();
    stdin()
        .read_line(&mut pause_time)
        .expect("Shit happened...");

    let pause_time_duration =
        std::time::Duration::new(0, pause_time.trim().parse::<u32>().unwrap() * 1000000);

    println!("Select stuff to send: a / s: ");
    let mut stuff = String::new();
    stdin().read_line(&mut stuff).expect("Shit happened...");

    loop {
        thread::sleep(pause_time_duration);
        match stuff.trim().as_ref() {
            "q" => {
                println!("graceful fall...");
                break;
            }
            "a" => client.send_request(JobType::DATA_PUSH_ACTION, &mut vec![3, 4, 5, 6]),
            "s" => client.send_request(JobType::PING, &mut vec![1, 4, 2, 6]),
            _ => {}
        }
    }

    println!("Exiting...");

    client.die();
}
