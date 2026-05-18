use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::dto::{EndpointDto, ParticipantDto, RegistrySnapshot, TopicDto};

#[derive(Default)]
pub struct Registry {
    participants: HashMap<String, ParticipantDto>,
    topics: HashMap<String, TopicDto>,
    writers: HashMap<String, EndpointDto>,
    readers: HashMap<String, EndpointDto>,
}

impl Registry {
    pub fn snapshot(&self) -> RegistrySnapshot {
        RegistrySnapshot {
            participants: self.participants.values().cloned().collect(),
            topics: self.topics.values().cloned().collect(),
            writers: self.writers.values().cloned().collect(),
            readers: self.readers.values().cloned().collect(),
        }
    }

    pub fn add_participant(&mut self, p: ParticipantDto) {
        self.participants.insert(p.guid.clone(), p);
    }

    pub fn remove_participant(&mut self, guid: &str) {
        self.participants.remove(guid);
        // Also drop any endpoints whose participant_guid matches.
        self.writers.retain(|_, e| e.participant_guid != guid);
        self.readers.retain(|_, e| e.participant_guid != guid);
    }

    pub fn add_topic(&mut self, t: TopicDto) {
        self.topics.insert(t.name.clone(), t);
    }

    pub fn remove_topic(&mut self, name: &str) {
        self.topics.remove(name);
    }

    pub fn add_writer(&mut self, e: EndpointDto) {
        self.writers.insert(e.guid.clone(), e);
    }

    pub fn remove_writer(&mut self, guid: &str) {
        self.writers.remove(guid);
    }

    pub fn add_reader(&mut self, e: EndpointDto) {
        self.readers.insert(e.guid.clone(), e);
    }

    pub fn remove_reader(&mut self, guid: &str) {
        self.readers.remove(guid);
    }
}

pub type SharedRegistry = Arc<Mutex<Registry>>;

pub fn new_shared() -> SharedRegistry {
    Arc::new(Mutex::new(Registry::default()))
}
