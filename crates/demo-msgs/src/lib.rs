use rustdds::Keyed;
use serde::{Deserialize, Serialize};

pub const DOMAIN_ID: u16 = 0;
pub const TOPIC_NAME: &str = "Chatter";
pub const TYPE_NAME: &str = "Chatter";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Chatter {
    pub publisher_id: u32,
    pub counter: u64,
    pub stamp_ns: u64,
    pub text: String,
}

impl Keyed for Chatter {
    type K = u32;
    fn key(&self) -> Self::K {
        self.publisher_id
    }
}
