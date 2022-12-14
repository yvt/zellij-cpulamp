use anyhow::{Context, Result};

use crate::{iter::BoxMiniIterator, slist};

#[derive(Debug, Default)]
pub struct System {
    cpus: slist::Link<Cpu>,
}

#[derive(Debug, Default)]
struct Cpu {
    stats: CpuStats,
    last_stats: CpuStats,
    tmp_stats: CpuStats,
}

#[derive(Debug, Default, Clone, Copy)]
struct CpuStats {
    total: u64,
    active: u64,
}

impl std::ops::SubAssign for CpuStats {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.total -= rhs.total;
        self.active -= rhs.active;
    }
}

impl std::ops::Sub for CpuStats {
    type Output = Self;

    #[inline]
    fn sub(mut self, rhs: Self) -> Self::Output {
        self -= rhs;
        self
    }
}

impl super::System for System {
    fn refresh_cpus(&mut self) -> Result<()> {
        let lines = crate::process::spawn_and_get_output(b"cat /proc/stat");
        let lines = std::str::from_utf8(&lines)?;
        let lines = lines.lines().filter(|line| {
            line.starts_with("cpu") && line.as_bytes().get(3).map_or(false, |b| b.is_ascii_digit())
        });

        let new_num_cpus = lines.clone().count();
        anyhow::ensure!(new_num_cpus != 0, "no CPUs found");

        slist::try_for_each_and_resize(
            &mut self.cpus,
            lines,
            |_| Cpu::default(),
            |cpu, stat_line| {
                (|| {
                    let stat_line = stat_line.split_once(" ").context("separator is absent")?.1;
                    let mut parts = [None::<u64>; 10];
                    for (part_out, part) in parts.iter_mut().zip(stat_line.split(" ")) {
                        *part_out = part.parse().ok();
                    }

                    let total: u64 = parts.iter().filter_map(|&x| x).sum();
                    let idle = parts[3].take().unwrap_or(0);
                    let iowait = parts[4].take().unwrap_or(0);
                    let idle = idle + iowait;

                    cpu.tmp_stats = CpuStats {
                        total,
                        active: total.saturating_sub(idle),
                    };
                    Ok(()) as Result<()>
                })()
                .with_context(|| format!("failed to parse line '{stat_line}'"))
            },
        )?;

        // Commit the result after the success is certain
        for cpu in slist::iter_mut(&mut self.cpus) {
            cpu.last_stats = cpu.stats;
            cpu.stats = cpu.tmp_stats;
        }

        Ok(())
    }

    fn num_cpus(&self) -> usize {
        slist::iter(&self.cpus).count()
    }

    fn iter_cpu_usage(&self) -> BoxMiniIterator<'_, f64> {
        Box::new(slist::iter(&self.cpus).map(|cpu| {
            let stats = cpu.stats - cpu.last_stats;
            stats.active as f64 / stats.total as f64
        }))
    }
}
