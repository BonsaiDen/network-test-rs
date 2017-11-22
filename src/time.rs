// Copyright (c) 2017 Ivo Wetzel

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


// STD Dependencies -----------------------------------------------------------
use std::iter;
use std::thread;
use std::time::{self, Instant, Duration};


// Internal Dependencies ------------------------------------------------------
use ::message::InternalMessage;


// Statics --------------------------------------------------------------------
static AVERAGE_SIZE: usize = 16;


// Timer Abstraction ----------------------------------------------------------
// TODO rename into state or something else?
pub struct Timer {
    tick: u8,
    ticks_per_second: u8,
    clock_shift: MovingAverage,
    last_wait: Instant,
    accumulated_wait: Duration,
    last_ping: Instant,
    average_rtt: MovingAverage
}

impl Timer {

    pub fn new(ticks_per_second: u8) -> Self {
        Self {
            tick: 0,
            ticks_per_second: ticks_per_second,
            clock_shift: MovingAverage::new(AVERAGE_SIZE),
            last_wait: Instant::now(),
            last_ping: Instant::now(),
            accumulated_wait: Duration::new(0, 0),
            average_rtt: MovingAverage::new(AVERAGE_SIZE)
        }
    }

    pub fn rtt(&self) -> f64 {
        self.average_rtt.get()
    }

    pub fn clock(&self) -> f64 {
        self.clock_shift.get()
    }

    pub fn reset(&mut self) {
        self.tick = 0;
        self.last_wait = Instant::now();
        self.accumulated_wait = Duration::new(0, 0);
        self.last_ping = Instant::now();
        self.clock_shift = MovingAverage::new(AVERAGE_SIZE);
        self.average_rtt = MovingAverage::new(AVERAGE_SIZE);
    }

    pub fn clone(&mut self) -> Self {
        Self {
            tick: self.tick,
            ticks_per_second: self.ticks_per_second,
            clock_shift: MovingAverage::new(AVERAGE_SIZE),
            last_wait: Instant::now(),
            last_ping: Instant::now(),
            accumulated_wait: Duration::new(0, 0),
            average_rtt: MovingAverage::new(AVERAGE_SIZE)
        }
    }

    // TODO callback for client side configuration?
    pub fn receive(&mut self, messages: Vec<InternalMessage>) -> Vec<InternalMessage> {

        let now = precise_time_ms();
        let mut outgoing = Vec::new();

        // Requests
        // TODO improve the logic here to send a lot of updates early on and then space out over
        // time
        // TODO work with a multiple of ticks instead of using a time based system
        let d = 1000 / u64::from(self.ticks_per_second) * 8;
        if self.last_ping.elapsed() > Duration::from_millis(d) {
            outgoing.push(InternalMessage::Ping(self.tick, now));
            self.last_ping = Instant::now();
        }

        // Responses
        for m in messages {
            match m {

                InternalMessage::Ping(tick, time) => {
                    outgoing.push(InternalMessage::Pong(tick, time, precise_time_ms()));
                },

                InternalMessage::Pong(tick, client_time, server_time) => {

                    let tick_duration = 1000 / u64::from(self.ticks_per_second);
                    let tick_diff = u64::from(self.tick.wrapping_sub(tick));

                    // Measure Round Trip Time
                    let expected_rtt = (tick_diff.saturating_sub(1) * tick_duration) as f64;
                    let actual_rtt = now.saturating_sub(client_time).saturating_sub(tick_duration) as f64;
                    let average_rtt = (expected_rtt + actual_rtt) / 2.0;

                    self.average_rtt.update(average_rtt, 1.0);

                    // Measure clock shift
                    if average_rtt <= self.average_rtt.get() * 1.5 {

                        let diff = (
                            (server_time as f64 - client_time as f64) +
                            (server_time as f64 - now as f64)

                        ) / 2.0;

                        self.clock_shift.update(diff, 0.5);

                    }

                }

            }
        }

        // Internal state
        self.tick = self.tick.wrapping_add(1);

        outgoing

    }

    pub fn sleep(&mut self) {

        // Calculate desired wait time
        let desired_wait = Duration::new(0, 1_000_000_000 / u32::from(self.ticks_per_second));

        // Calculate additional time taken by external logic
        self.accumulated_wait += self.last_wait.elapsed();

        // If the accumulated wait is lower than the desired_wait wait, simply subtract it
        if self.accumulated_wait <= desired_wait {
            thread::sleep(desired_wait - self.accumulated_wait);
            self.accumulated_wait = Duration::new(0, 0);

        // Otherwise reduce the accumulated wait by desired_wait and do not sleep at all
        } else {
            self.accumulated_wait -= desired_wait;
        }

        self.last_wait = Instant::now();

    }

}

// Utilites -------------------------------------------------------------------
fn precise_time_ms() -> u64 {

    let dur = match time::SystemTime::now().duration_since(time::UNIX_EPOCH) {
        Ok(dur) => dur,
        Err(err) => err.duration(),
    };

    dur.as_secs() * 1000 + u64::from(dur.subsec_nanos() / 1_000_000)

}

struct MovingAverage {
    size: usize,
    index: usize,
    used: usize,
    average: f64,
    values: Vec<f64>
}

impl MovingAverage {

    fn new(size: usize) -> Self {
        Self {
            size: size,
            index: 0,
            used: 0,
            average: 0.0,
            values: iter::repeat(0.0f64).take(size).collect()
        }
    }

    fn get(&self) -> f64 {
        self.average
    }

    fn update(&mut self, value: f64, ratio: f64) {

        self.values[self.index] = self.average * (1.0 - ratio) + value * ratio;
        self.index += 1;

        if self.used < self.size {
            self.used += 1;
        }

        if self.index == self.size {
            self.index = 0;
        }

        let value: f64 = self.values.iter().sum();
        self.average = value / self.used as f64

    }

}

