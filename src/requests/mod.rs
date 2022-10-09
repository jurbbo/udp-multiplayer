pub mod job;
pub mod jobs;
pub mod jobtype;
pub mod jobworkers;

use std::time::Instant;

pub enum JobAction {
    REMOVE,
    ADD,
    INCFAILED,
    INCHANDLED,
}

#[derive(Clone)]
pub struct Job {
    handle: u8,
    pub start_instant: Instant,
    pub pending: bool,
    pub raw_data: Vec<u8>,
    pub job_type: JobType,
    pub requested_count: i8,
    job_should_complite: u128,
}

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
