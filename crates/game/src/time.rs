use ecs::core::Resource;
use std::time::{Duration, Instant};

pub struct Time {
    start: Instant,
    last_elapsed: Instant,
    elapsed: Duration,
    delta: Duration,
    max_delta: Option<Duration>,
    scale: f32,
}

impl Time {
    pub fn new() -> Self {
        let start = Instant::now();
        Self {
            start,
            last_elapsed: start,
            elapsed: Duration::ZERO,
            delta: Duration::ZERO,
            max_delta: None,
            scale: 1.0,
        }
    }

    pub fn cap_frame_rate(mut self, rate: u32) -> Self {
        self.max_delta = Some(Duration::from_secs_f64(1.0 / rate as f64));
        self
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        self.delta = match self.max_delta {
            Some(max) => max.min(now - self.last_elapsed).mul_f32(self.scale),
            None => (now - self.last_elapsed).mul_f32(self.scale),
        };

        self.last_elapsed = now;
        self.elapsed += self.delta;
    }

    pub fn start(&self) -> Instant {
        self.start
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    pub fn elapsed_secs(&self) -> f32 {
        self.elapsed.as_secs_f32()
    }

    pub fn delta(&self) -> Duration {
        self.delta
    }

    pub fn delta_secs(&self) -> f32 {
        self.delta.as_secs_f32()
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }
}

impl Resource for Time {}
