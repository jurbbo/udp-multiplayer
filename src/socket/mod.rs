use std::net::IpAddr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub trait SocketCombatible {
    fn is_running(&self) -> bool;
    fn get_port(&self) -> Option<u16>;
    fn get_ip(&self) -> Option<IpAddr>;
    fn get_is_running_atomic(&self) -> Arc<AtomicBool>;
}
