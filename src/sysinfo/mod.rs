use anyhow::Result;

use crate::iter::BoxMiniIterator;

mod linux;

pub trait System {
    fn refresh_cpus(&mut self) -> Result<()>;
    fn num_cpus(&self) -> usize;
    fn iter_cpu_usage(&self) -> BoxMiniIterator<'_, f64>;
}

#[inline]
pub fn current_system() -> Option<Box<dyn System>> {
    // TODO: support other systems
    Some(Box::new(linux::System::default()))
}
