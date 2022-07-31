use anyhow::Result;

mod linux;

pub trait System {
    fn refresh_cpus(&mut self) -> Result<()>;
    fn num_cpus(&self) -> usize;
    fn cpu_usage(&self, cpu_i: usize) -> f64;
}

#[inline]
pub fn current_system() -> Option<Box<dyn System>> {
    // TODO: support other systems
    Some(Box::new(linux::System::default()))
}
