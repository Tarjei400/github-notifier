use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::time::{Duration, Instant};



#[derive(Debug)]
struct SnoozeNotifications {
    pub map: HashMap<String, Duration>,

    //For unsnoozing items
    pub queue: BinaryHeap<Reverse<(Instant, String)>>,

}

impl SnoozeNotifications {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            queue: BinaryHeap::new(),
        }
    }

    pub fn snooze(&mut self, repository: String, duration: Duration) {
        self.map.insert(repository.clone(), duration);
        let expires_at = Instant::now() + duration;
        self.queue.push(Reverse((expires_at,repository)))
    }

    pub fn is_snoozed(&self, key: &str) -> bool {
        match self.map.get(key) {
            Some(expire) => true, //If it exists in map it means it is snoozed, let job to clean snoozing
            None => false
        }
    }

    pub fn unsnooze_expires(&mut self) {
        loop {
            let next_expiring = self.queue.peek().cloned();
            match (next_expiring) {
                Some(Reverse((expires_at, repository))) => {
                    if expires_at <= Instant::now() {
                        self.queue.pop();
                        self.map.remove(&repository);
                    } else {
                        break;
                    }
                },
                None => break
            }
        }
    }

    pub fn handle_snooze_message(&mut self) {

    }

}

