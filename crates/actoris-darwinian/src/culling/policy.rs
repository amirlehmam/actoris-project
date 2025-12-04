//! Culling policy: <0.7 fitness for 2 epochs = cull

pub struct CullingPolicy {
    threshold: f64,
    grace_epochs: u64,
}

impl CullingPolicy {
    pub fn new(threshold: f64, grace_epochs: u64) -> Self {
        Self { threshold, grace_epochs }
    }

    pub fn should_cull(&self, fitness: f64, epochs_below: u64) -> bool {
        fitness < self.threshold && epochs_below >= self.grace_epochs
    }
}
