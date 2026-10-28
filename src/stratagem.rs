use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    pub fn arrow(self) -> &'static str {
        match self {
            Dir::Up => "▲",
            Dir::Down => "▼",
            Dir::Left => "◀",
            Dir::Right => "▶",
        }
    }
}

pub struct Stratagem {
    pub name: &'static str,
    pub code: &'static [Dir],
}

use Dir::*;

/// Built-in stratagems. Names and codes are our own, not taken from Helldivers.
pub const BUILTIN: &[Stratagem] = &[
    Stratagem {
        name: "Orbital Toaster",
        code: &[Up, Right, Down, Down, Right],
    },
    Stratagem {
        name: "Supply Llama",
        code: &[Down, Down, Up, Right],
    },
    Stratagem {
        name: "Emergency Confetti",
        code: &[Down, Up, Left, Down, Up, Right, Down],
    },
    Stratagem {
        name: "Tactical Rubber Duck",
        code: &[Left, Down, Right, Up],
    },
    Stratagem {
        name: "Panic Button Deluxe",
        code: &[Up, Up, Down, Down, Left, Right],
    },
    Stratagem {
        name: "Portable Thunderstorm",
        code: &[Right, Down, Up, Right, Down],
    },
    Stratagem {
        name: "Shield of Mild Concern",
        code: &[Down, Left, Left, Right],
    },
    Stratagem {
        name: "Airborne Pizza Drop",
        code: &[Up, Right, Right, Down, Left],
    },
    Stratagem {
        name: "Compact Black Hole",
        code: &[Right, Right, Up, Down, Down, Up, Right],
    },
    Stratagem {
        name: "Very Large Hammer",
        code: &[Down, Down, Right, Up, Left],
    },
    Stratagem {
        name: "Reinforcement Pigeon",
        code: &[Up, Down, Right, Left, Up],
    },
    Stratagem {
        name: "Gentle Napalm",
        code: &[Right, Right, Left, Down, Up],
    },
    Stratagem {
        name: "Recon Drone Buddy",
        code: &[Left, Up, Right, Down, Left, Up],
    },
    Stratagem {
        name: "Uplink Calibrator",
        code: &[Up, Up, Left, Right, Left, Right, Down],
    },
    Stratagem {
        name: "Bubble Wrap Barrier",
        code: &[Down, Right, Down, Right, Up, Up],
    },
    Stratagem {
        name: "Heavy Metal Jukebox",
        code: &[Left, Left, Right, Right, Up, Down, Up, Down],
    },
];

/// How many stratagems are shown ahead, current one included.
const QUEUE: usize = 3;
/// How long the sequence stays red after a wrong input.
pub const FLASH: Duration = Duration::from_millis(300);

/// A timed round: enter as many stratagem codes as possible. A wrong input resets the current code.
pub struct Drill {
    pub secs: u64,
    /// Indexes into BUILTIN; the first is the one being entered.
    pub queue: Vec<usize>,
    pub progress: usize,
    pub streak: u32,
    pub best_streak: u32,
    pub inputs: u32,
    pub errors: u32,
    /// Seconds spent on each completed stratagem.
    pub times: Vec<f64>,
    pub last_error: Option<Instant>,
    start: Option<Instant>,
    end: Option<Instant>,
    current_since: f64,
    clean: bool,
}

impl Drill {
    pub fn new(secs: u64) -> Self {
        let mut d = Drill {
            secs,
            queue: Vec::new(),
            progress: 0,
            streak: 0,
            best_streak: 0,
            inputs: 0,
            errors: 0,
            times: Vec::new(),
            last_error: None,
            start: None,
            end: None,
            current_since: 0.0,
            clean: true,
        };
        while d.queue.len() < QUEUE {
            d.push_random();
        }
        d
    }

    fn push_random(&mut self) {
        loop {
            let i = rand::random_range(0..BUILTIN.len());
            if self.queue.last() != Some(&i) {
                self.queue.push(i);
                return;
            }
        }
    }

    pub fn current(&self) -> &'static Stratagem {
        &BUILTIN[self.queue[0]]
    }

    pub fn started(&self) -> bool {
        self.start.is_some()
    }

    pub fn finished(&self) -> bool {
        self.end.is_some()
    }

    pub fn elapsed(&self, now: Instant) -> f64 {
        match self.start {
            Some(s) => (self.end.unwrap_or(now) - s).as_secs_f64(),
            None => 0.0,
        }
    }

    pub fn tick(&mut self, now: Instant) {
        if let (Some(start), None) = (self.start, self.end)
            && now - start >= Duration::from_secs(self.secs)
        {
            self.end = Some(start + Duration::from_secs(self.secs));
        }
    }

    pub fn input(&mut self, dir: Dir, now: Instant) {
        if self.finished() {
            return;
        }
        let start = *self.start.get_or_insert(now);
        self.inputs += 1;
        let code = self.current().code;
        if code[self.progress] != dir {
            self.errors += 1;
            self.progress = 0;
            self.streak = 0;
            self.clean = false;
            self.last_error = Some(now);
            return;
        }
        self.progress += 1;
        if self.progress == code.len() {
            let t = (now - start).as_secs_f64();
            self.times.push(t - self.current_since);
            self.current_since = t;
            if self.clean {
                self.streak += 1;
                self.best_streak = self.best_streak.max(self.streak);
            }
            self.clean = true;
            self.progress = 0;
            self.queue.remove(0);
            self.push_random();
        }
    }

    pub fn accuracy(&self) -> f64 {
        if self.inputs == 0 {
            return 100.0;
        }
        (self.inputs - self.errors) as f64 * 100.0 / self.inputs as f64
    }

    pub fn stats(&self, now: Instant) -> DrillStats {
        let duration = self.elapsed(now);
        let done = self.times.len();
        DrillStats {
            completed: done,
            per_minute: if duration > 0.0 {
                done as f64 * 60.0 / duration
            } else {
                0.0
            },
            inputs_per_minute: if duration > 0.0 {
                self.inputs as f64 * 60.0 / duration
            } else {
                0.0
            },
            accuracy: self.accuracy(),
            avg_time: if done > 0 {
                self.times.iter().sum::<f64>() / done as f64
            } else {
                0.0
            },
            best_streak: self.best_streak,
            errors: self.errors,
            duration,
            times: self.times.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DrillStats {
    pub completed: usize,
    pub per_minute: f64,
    pub inputs_per_minute: f64,
    pub accuracy: f64,
    pub avg_time: f64,
    pub best_streak: u32,
    pub errors: u32,
    pub duration: f64,
    pub times: Vec<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enter_current(d: &mut Drill, at: Instant) {
        for &dir in d.current().code {
            d.input(dir, at);
        }
    }

    fn wrong(d: &mut Drill) -> Dir {
        let right = d.current().code[d.progress];
        [Up, Down, Left, Right].into_iter().find(|&x| x != right).unwrap()
    }

    #[test]
    fn completing_codes_advances_queue_and_streak() {
        let mut d = Drill::new(30);
        let start = Instant::now();
        let next = d.queue[1];
        enter_current(&mut d, start);
        assert_eq!(d.queue[0], next);
        assert_eq!(d.queue.len(), QUEUE);
        enter_current(&mut d, start + Duration::from_secs(2));
        assert_eq!(d.times, vec![0.0, 2.0]);
        assert_eq!(d.streak, 2);
        assert_eq!(d.accuracy(), 100.0);
    }

    #[test]
    fn wrong_input_resets_code_and_streak() {
        let mut d = Drill::new(30);
        let now = Instant::now();
        enter_current(&mut d, now);
        let first = d.current().code[0];
        d.input(first, now);
        let w = wrong(&mut d);
        d.input(w, now);
        assert_eq!((d.progress, d.streak, d.errors), (0, 0, 1));
        // Finishing it after a mistake doesn't count toward the streak.
        enter_current(&mut d, now);
        assert_eq!((d.streak, d.best_streak), (0, 1));
    }

    #[test]
    fn time_limit_ends_round() {
        let mut d = Drill::new(10);
        let start = Instant::now();
        enter_current(&mut d, start);
        d.tick(start + Duration::from_secs(11));
        assert!(d.finished());
        let s = d.stats(start + Duration::from_secs(99));
        assert_eq!((s.completed, s.duration), (1, 10.0));
        assert_eq!(s.per_minute, 6.0);
    }

    #[test]
    fn builtin_codes_are_unique_and_not_empty() {
        for (i, a) in BUILTIN.iter().enumerate() {
            assert!(!a.code.is_empty());
            assert!(
                BUILTIN[i + 1..]
                    .iter()
                    .all(|b| b.code != a.code && b.name != a.name)
            );
        }
    }
}
