use std::thread;
use std::time::{Duration, Instant};

pub struct Ticker {
    tick_start: Instant,
    min_tick_time: Duration,
    last_tps_measurement: Instant,
    tick_count: usize,
    tps: usize,
}

impl Ticker {
    pub fn new(min_tick_time: Duration) -> Self {
        Self {
            tick_start: Instant::now(),
            min_tick_time,
            last_tps_measurement: Instant::now(),
            tick_count: 0,
            tps: 0,
        }
    }

    pub fn start_tick(&mut self) {
        self.tick_start = Instant::now();
    }
    pub fn end_tick(&mut self) {
        let tick_end = Instant::now();
        let tick_duration = tick_end.duration_since(self.tick_start);
        if tick_duration < self.min_tick_time {
            thread::sleep(self.min_tick_time - tick_duration);
        }

        self.tick_count += 1;
        let since_last_tps_measurement = Instant::now().duration_since(self.last_tps_measurement);
        if since_last_tps_measurement.as_millis() > 1000 {
            self.tps = self.tick_count;
            self.tick_count = 0;
            self.last_tps_measurement = Instant::now();
        }
    }
    pub fn get_tps(&self) -> usize {
        self.tps
    }
    pub fn set_min_tick_time(&mut self, t: Duration) {
        self.min_tick_time = t;
    }
}
