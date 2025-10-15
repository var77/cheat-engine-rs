use std::process::{Child, Command, Stdio};

pub struct ChildGuard(pub Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        // Try to kill the process if still running
        if let Ok(Some(_)) = self.0.try_wait() {
            // already exited
            return;
        }
        let _ = self.0.kill();
        let _ = self.0.wait(); // reap zombie
    }
}
