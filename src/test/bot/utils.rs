pub mod assert;

use std::{
    fmt::Debug,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

#[derive(Default, Debug)]
pub struct Counters {
    get_calculator_state: AtomicU64,
}

impl Counters {
    pub const fn new() -> Self {
        Self {
            get_calculator_state: AtomicU64::new(0),
        }
    }

    pub fn inc_get_calculator_state(&self) {
        self.get_calculator_state.fetch_add(1, Ordering::Relaxed);
    }

    pub fn reset_get_calculator_state(&self) -> u64 {
        self.get_calculator_state.swap(0, Ordering::Relaxed)
    }
}

#[derive(Debug)]
pub struct Timer {
    previous: Instant,
    time: Duration,
}

impl Timer {
    pub fn new(time: Duration) -> Self {
        Self {
            previous: Instant::now(),
            time,
        }
    }

    pub fn passed(&mut self) -> bool {
        if self.previous.elapsed() >= self.time {
            self.previous = Instant::now();
            true
        } else {
            false
        }
    }
}

pub struct AvgTime {
    previous: Instant,
    total: u64,
    counter: u64,
    calculate_avg_when_couter: u64,
    current_avg: Duration,
}

impl AvgTime {
    pub fn new(calculate_avg_when_couter: u64) -> Self {
        Self {
            previous: Instant::now(),
            total: 0,
            counter: 0,
            calculate_avg_when_couter,
            current_avg: Duration::from_micros(0),
        }
    }

    pub fn track(&mut self) {
        self.previous = Instant::now();
    }

    pub fn complete(&mut self) {
        let time = self.previous.elapsed();
        self.total += time.as_micros() as u64;
        self.counter += 1;

        if self.counter >= self.calculate_avg_when_couter {
            self.current_avg = Duration::from_micros(self.total / self.counter);

            self.counter = 0;
            self.total = 0;
        }
    }

    pub fn current_avg(&self) -> Duration {
        self.current_avg
    }
}
