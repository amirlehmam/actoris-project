//! PID controller for resource allocation

use pid::Pid;

pub struct DarwinianController {
    pid: Pid<f64>,
    target: f64,
}

impl DarwinianController {
    pub fn new(target: f64, kp: f64, ki: f64, kd: f64) -> Self {
        let mut pid = Pid::new(target, 100.0);
        pid.p(kp, 100.0);
        pid.i(ki, 100.0);
        pid.d(kd, 100.0);

        Self { pid, target }
    }

    pub fn target(&self) -> f64 {
        self.target
    }

    pub fn compute(&mut self, measurement: f64) -> f64 {
        self.pid.next_control_output(measurement).output
    }
}
