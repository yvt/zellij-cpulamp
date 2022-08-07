//! Plugin entry point
use num_integer::div_ceil;
use std::time::Instant;
use zellij_tile::prelude::*;
use zellij_tile_utils::style;

use zellij_cpulamp::{slist, sysinfo};

struct State {
    mode_info: ModeInfo,
    sysinfo: Box<dyn sysinfo::System>,
    elapsed_since_last_frame_us: u32,
    elapsed_since_last_measure_f: u32,
    last_timeout: Instant,
    cpus: slist::Link<CpuState>,
    output_buffer: String,
}

struct CpuState {
    charge: u32,
    rate: u32,
    /// Indicates whether this CPU's usage indicator is active for the current
    /// frame.
    lit: bool,
}

register_plugin!(State);

/// The unit of time used for various operations in this plugin.
const FRAME_INTERVAL_US: u32 = 200_000;

/// The frequency of measuring the latest CPU usage, measured in [frames]
/// (FRAME_INTERVAL_US).
const MEASURE_INTERVAL_F: u32 = 5;

impl Default for State {
    fn default() -> Self {
        Self {
            mode_info: Default::default(),
            sysinfo: sysinfo::current_system().expect("unsupported system"),
            // Instantly start a new frame
            elapsed_since_last_frame_us: FRAME_INTERVAL_US,
            // Instantly perform the first measurement
            elapsed_since_last_measure_f: MEASURE_INTERVAL_F,
            cpus: None,
            last_timeout: Instant::now(),
            output_buffer: String::new(),
        }
    }
}

impl State {
    fn on_timeout(&mut self) {
        let now = Instant::now();
        let elapsed_us = now
            .checked_duration_since(self.last_timeout)
            .map(|d| d.as_micros())
            .unwrap_or(0);
        self.last_timeout = now;

        self.elapsed_since_last_frame_us = self
            .elapsed_since_last_frame_us
            .saturating_add(elapsed_us.try_into().unwrap_or(u32::MAX));
        let num_frames = self.elapsed_since_last_frame_us / FRAME_INTERVAL_US;
        self.elapsed_since_last_frame_us -= num_frames * FRAME_INTERVAL_US;

        self.elapsed_since_last_measure_f += num_frames;

        if self.elapsed_since_last_measure_f >= MEASURE_INTERVAL_F {
            self.elapsed_since_last_measure_f = 0;

            // Update CPUs
            match self.sysinfo.refresh_cpus() {
                Ok(()) => {
                    slist::resize_with(&mut self.cpus, self.sysinfo.num_cpus(), |_| CpuState {
                        charge: 0,
                        rate: 0,
                        lit: false,
                    });
                    for (cpu, cpu_usage) in
                        slist::iter_mut(&mut self.cpus).zip(self.sysinfo.iter_cpu_usage())
                    {
                        cpu.rate = (cpu_usage * u32::MAX as f64) as u32;
                    }
                }
                Err(e) => {
                    eprintln!("Failed to update CPU statistics: {e:?}");
                }
            }
        }

        // The next timeout period
        let mut timeout_f = MEASURE_INTERVAL_F - self.elapsed_since_last_measure_f;

        for cpu in slist::iter_mut(&mut self.cpus) {
            if num_frames > 0 {
                (cpu.charge, cpu.lit) = cpu
                    .charge
                    .overflowing_add(cpu.rate.saturating_mul(num_frames));
            }

            // Guess the frame on which `cpu.lit` will change
            let change_f = if cpu.lit {
                // When will it stop overflowing?
                let antirate = cpu.rate.wrapping_neg();
                (cpu.charge / antirate).saturating_add(1)
            } else {
                // When will it overflow?
                if cpu.rate == 0 {
                    // Never
                    continue;
                }
                let remaining_charge = cpu.charge.max(1).wrapping_neg();
                // Rounding-up division
                div_ceil(remaining_charge, cpu.rate)
            };
            timeout_f = timeout_f.min(change_f);
        }

        assert_ne!(timeout_f, 0);
        let timeout_us = (timeout_f - 1) as u32 * FRAME_INTERVAL_US
            + (FRAME_INTERVAL_US - self.elapsed_since_last_frame_us);
        set_timeout(timeout_us as f64 * 1.0e-6);
    }
}

impl ZellijPlugin for State {
    fn load(&mut self) {
        set_selectable(false);
        subscribe(&[EventType::Timer, EventType::ModeUpdate]);
        self.last_timeout = Instant::now();
        self.on_timeout();
    }

    fn update(&mut self, event: Event) {
        match event {
            Event::ModeUpdate(mode_info) => self.mode_info = mode_info,
            Event::Timer(_elapsed_secs) => {
                // Don't use `_elapsed_secs` because it doesn't actually
                // represent the elapsed time since the last `Event::Timer`
                // event.
                //
                // `_elapsed_secs` would actually represent the elapsed time if
                // there were exactly one series of timeout events sustained by
                // `set_timeout` calls in the timeout handler. In reality,
                // however, it appears that Zellij loads two instances of this
                // plugin (bug?), calls `load()` on both, and for some reason
                // delivers both of the two initial timeout events only to the
                // second instance, setting off two series of timeout events.
                self.on_timeout();
            }
            _ => {}
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        let Self {
            cpus,
            mode_info,
            output_buffer,
            ..
        } = self;

        output_buffer.clear();

        let num_cpus = slist::iter(cpus).count();
        let mut cpu_states = slist::iter(cpus).map(|c| c.lit);
        let area = rows * cols;
        if area >= num_cpus {
            // Sparse (one cpu per cell)
            for _ in 0..rows {
                for _ in 0..cols {
                    if cpu_states.next() == Some(true) {
                        output_buffer.push_str("•");
                    } else {
                        output_buffer.push_str(" ");
                    }
                }
                output_buffer.push_str("\n");
            }
        } else {
            // Dense (8n cpus per cell)
            let group_len = div_ceil(num_cpus, area * 8);
            for _ in 0..rows {
                for _ in 0..cols {
                    let bitmap = (0..8).fold(0u8, |acc, bit| {
                        let lit = (0..group_len)
                            .map(|_| cpu_states.next().unwrap_or(false))
                            .fold(false, |x, y| x | y);
                        acc | ((lit as u8) << bit)
                    });
                    let braille = zellij_cpulamp::bitmap_to_braille(bitmap);
                    output_buffer.push(braille);
                }
                output_buffer.push_str("\n");
            }
        }

        output_buffer.pop();

        let (bg, fg) = match mode_info.style.colors.theme_hue {
            ThemeHue::Light => (mode_info.style.colors.white, mode_info.style.colors.orange),
            ThemeHue::Dark => (mode_info.style.colors.black, mode_info.style.colors.orange),
        };
        print!("{}", style!(fg, bg).paint(output_buffer.as_str()));
    }
}
