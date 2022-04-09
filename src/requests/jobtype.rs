pub enum JobType {
    DATA_PUSH_ACTION,
    DATA_PUSH_RECEIVED,
    DATA_REQUEST,
    PLAYER_ENTER,
    PLAYER_LEAVE,
    PING,
}

pub fn transfer_job_type_to_byte(job_type: &JobType) -> u8 {
    match job_type {
        JobType::DATA_PUSH_ACTION => 1,
        JobType::DATA_PUSH_RECEIVED => 2,
        JobType::DATA_REQUEST => 3,
        JobType::PLAYER_ENTER => 4,
        JobType::PLAYER_LEAVE => 5,
        JobType::PING => 6,
    }
}

pub fn tranfer_job_type_from_number(job_type_number: &u8) -> Option<JobType> {
    match job_type_number {
        1 => Some(JobType::DATA_PUSH_ACTION),
        2 => Some(JobType::DATA_PUSH_RECEIVED),
        3 => Some(JobType::DATA_REQUEST),
        4 => Some(JobType::PLAYER_ENTER),
        5 => Some(JobType::PLAYER_LEAVE),
        6 => Some(JobType::PING),
        __ => None,
    }
}
