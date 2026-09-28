//! Runs the global solver off the UI thread. Dropping the task cancels it.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::game::SolverJob;
use crate::solver::{self, Cancelled, Solution};

pub struct HelpTask {
    job: SolverJob,
    cancel: Arc<AtomicBool>,
    result: Receiver<Result<Solution, Cancelled>>,
}

impl HelpTask {
    /// `on_done` runs on the worker thread once the answer is ready, so the
    /// UI can wake up for it.
    pub fn spawn(job: SolverJob, on_done: impl FnOnce() + Send + 'static) -> Self {
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let puzzle = job.puzzle.clone();
        let flag = Arc::clone(&cancel);
        thread::spawn(move || {
            let _ = tx.send(solver::solve(&puzzle, &flag));
            on_done();
        });
        Self {
            job,
            cancel,
            result: rx,
        }
    }

    pub fn job(&self) -> &SolverJob {
        &self.job
    }

    /// `None` while still working. A worker that died counts as cancelled.
    pub fn poll(&self) -> Option<Result<Solution, Cancelled>> {
        match self.result.try_recv() {
            Ok(answer) => Some(answer),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(Cancelled)),
        }
    }
}

impl Drop for HelpTask {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
