pub type JobType = (ServerJob, ClientJob);

#[derive(Clone)]
pub enum ClientJob {
    NoClientAction = 0,
    DataPushRequest = 1,
    DataRequest = 2,
    PlayerEnterRequest = 3,
    PlayerLeaveRequest = 4,
    PingRequest = 5,
}

#[derive(Clone)]
pub enum ServerJob {
    NoServerAction = 0,
    DataPush = 1,
    DataPushDoneResponse = 2,
    DataResponse = 3,
    PlayerCreatedResponse = 4,
    PlayerEnterPush = 5,
    PlayerLeaveResponse = 6,
    PlayerLeavePush = 7,
    PongResponse = 8,
}

impl ClientJob {
    pub fn as_string() -> Vec<&'static str> {
        vec![
            "NoClientAction",
            "DataPushRequest",
            "DataRequest",
            "PlayerEnterRequest",
            "PlayerLeaveRequest",
            "PingRequest",
        ]
    }
}

impl ServerJob {
    pub fn as_string() -> Vec<&'static str> {
        vec![
            "NoServerAction",
            "DataPush",
            "DataPushDoneResponse",
            "DataResponse",
            "PlayerCreatedResponse",
            "PlayerEnterPush",
            "PlayerLeaveResponse",
            "PlayerLeavePush",
            "PongResponse",
        ]
    }
}

pub fn get_job_bytes(job_type: &JobType) -> (u8, u8) {
    (
        match job_type.0 {
            ServerJob::NoServerAction => 0,
            ServerJob::DataPush => 1,
            ServerJob::DataPushDoneResponse => 2,
            ServerJob::DataResponse => 3,
            ServerJob::PlayerCreatedResponse => 4,
            ServerJob::PlayerEnterPush => 5,
            ServerJob::PlayerLeaveResponse => 6,
            ServerJob::PlayerLeavePush => 7,
            ServerJob::PongResponse => 8,
        },
        match job_type.1 {
            ClientJob::NoClientAction => 0,
            ClientJob::DataPushRequest => 1,
            ClientJob::DataRequest => 2,
            ClientJob::PlayerEnterRequest => 3,
            ClientJob::PlayerLeaveRequest => 4,
            ClientJob::PingRequest => 5,
        },
    )
}

pub fn get_job_single_byte(job_type: &JobType) -> u8 {
    let job_bytes = get_job_bytes(job_type);
    //println!("{}, {}", job_bytes.0, job_bytes.1);
    create_job_type_byte(&job_bytes)
}

fn create_job_type_byte(server_client: &(u8, u8)) -> u8 {
    let (server, client) = server_client;
    //println!("{:#b}", server);
    //println!("{:#b}", client);

    let server_left_bits = server << 4;
    //println!("{:#b}", server_left_bits);
    server_left_bits | client
}

pub fn get_job_type(server_client: &(u8, u8)) -> Option<JobType> {
    let (server, client) = server_client;

    let server_job = match server {
        0 => Some(ServerJob::NoServerAction),
        1 => Some(ServerJob::DataPush),
        2 => Some(ServerJob::DataPushDoneResponse),
        3 => Some(ServerJob::DataResponse),
        4 => Some(ServerJob::PlayerCreatedResponse),
        5 => Some(ServerJob::PlayerEnterPush),
        6 => Some(ServerJob::PlayerLeaveResponse),
        7 => Some(ServerJob::PlayerLeavePush),
        8 => Some(ServerJob::PongResponse),
        __ => None,
    };

    let client_job = match client {
        0 => Some(ClientJob::NoClientAction),
        1 => Some(ClientJob::DataPushRequest),
        2 => Some(ClientJob::DataRequest),
        3 => Some(ClientJob::PlayerEnterRequest),
        4 => Some(ClientJob::PlayerLeaveRequest),
        5 => Some(ClientJob::PingRequest),
        __ => None,
    };

    if server_job.is_none() || client_job.is_none() {
        return None;
    }

    Some((server_job.unwrap(), client_job.unwrap()))
}
