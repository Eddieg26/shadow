
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Init,
    Start,
    Execute,
    End,
    Shutdown,
}

pub struct Schedule {
    
}