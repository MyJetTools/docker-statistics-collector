use std::collections::VecDeque;

use crate::METRICS_HISTORY_SIZE;

#[derive(Clone)]
pub struct MetricsHistory<T: Copy> {
    data: VecDeque<T>,
}

impl<T: Copy> MetricsHistory<T> {
    pub fn new() -> Self {
        Self {
            data: VecDeque::new(),
        }
    }

    pub fn add(&mut self, value: T) {
        while self.data.len() >= METRICS_HISTORY_SIZE {
            self.data.pop_front();
        }

        self.data.push_back(value);
    }

    pub fn get_snapshot(&self) -> Vec<T> {
        let mut result = Vec::with_capacity(self.data.len());

        for itm in self.data.iter() {
            result.push(itm.clone());
        }

        result
    }
}
