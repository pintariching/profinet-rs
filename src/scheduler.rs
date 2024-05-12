use crate::{constants::MAX_SCHEDULER_TASKS, fspm::app::App, PNet};

#[derive(Clone, Copy)]
pub struct Task<T: TaskCallback + Copy> {
    name: &'static str,
    run_at: usize,
    task: T,
}

pub trait TaskCallback {
    fn callback<T: App + Copy, U: TaskCallback + Copy>(&mut self, pnet: &mut PNet<T, U>);
}

pub struct Scheduler<T: TaskCallback + Copy> {
    tasks: [Option<Task<T>>; MAX_SCHEDULER_TASKS],
}

impl<T> Scheduler<T>
where
    T: TaskCallback + Copy,
{
    pub fn new(tick_interval: usize) -> Self {
        if tick_interval == 0 {
            defmt::panic!("Tick interval must be more than 0");
        }

        Self {
            tasks: [None; MAX_SCHEDULER_TASKS],
        }
    }

    pub fn add_task(&mut self, name: &'static str, delay: usize, callback: T, current_time: usize) {
        for i in 0..MAX_SCHEDULER_TASKS {
            match self.tasks[i] {
                None => {
                    let new_task = Task {
                        name,
                        run_at: current_time + delay,
                        task: callback,
                    };

                    self.tasks[i] = Some(new_task);
                }
                _ => (),
            }
        }
    }

    pub fn tick<U: App + Copy>(&mut self, pnet: &mut PNet<U, T>, current_time: usize) {
        for i in 0..MAX_SCHEDULER_TASKS {
            if let Some(task) = &mut self.tasks[i] {
                if current_time >= task.run_at {
                    task.task.callback(pnet);
                    self.tasks[i] = None;
                }
            }
        }
    }
}
