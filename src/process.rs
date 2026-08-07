extern crate alloc;

use crate::arch::context::{initialize_stack, switch_context};
use crate::error::{Error, Result};
use crate::memory::Pages;
use crate::mutex::Mutex;

const KERNEL_STACK_PAGES: usize = 2;
const PROCESSES_MAX: usize = 8;

pub type ProcessEntry = extern "C" fn() -> isize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Idle,
    Runnable,
    Exited(isize),
}

#[derive(Debug)]
pub struct Process {
    pid: usize,
    parent_pid: Option<usize>,
    state: ProcessState,
    sp: usize,
    stack: Option<Pages>,
    entry: Option<ProcessEntry>,
}

impl Process {
    pub fn new(pid: usize, parent_pid: Option<usize>, entry: ProcessEntry) -> Option<Self> {
        let mut stack = Pages::alloc(KERNEL_STACK_PAGES)?;
        let sp = initialize_stack(&mut stack, process_entry_trampoline)?;

        Some(Self {
            pid,
            parent_pid,
            state: ProcessState::Runnable,
            sp,
            stack: Some(stack),
            entry: Some(entry),
        })
    }

    fn new_idle() -> Self {
        Self {
            pid: 0,
            parent_pid: None,
            state: ProcessState::Idle,
            sp: 0,
            stack: None,
            entry: None,
        }
    }

    pub fn pid(&self) -> usize {
        self.pid
    }

    pub fn state(&self) -> ProcessState {
        self.state
    }

    pub fn stack_pointer(&self) -> usize {
        self.sp
    }

    pub fn stack_start(&self) -> Option<usize> {
        self.stack.as_ref().map(|stack| stack.start_address())
    }

    pub fn stack_end(&self) -> Option<usize> {
        self.stack
            .as_ref()
            .map(|stack| stack.start_address() + stack.size())
    }

    /// Switches execution from this process to `next`.
    ///
    /// # Safety
    ///
    /// - Both processes must remain alive until execution switches back.
    /// - Their kernel stacks must not be freed.
    /// - `next.sp` must point to a valid ContextFrame.
    pub unsafe fn switch_to(&mut self, next: &Process) {
        let previous_sp = core::ptr::addr_of_mut!(self.sp);
        let next_sp = core::ptr::addr_of!(next.sp);

        unsafe {
            switch_context(previous_sp, next_sp);
        }
    }
}

struct SwitchContext {
    previous_sp_ptr: *mut usize,
    next_sp_ptr: *const usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitResult {
    Running,
    Exited(isize),
}

#[derive(Debug)]
pub struct ProcessManager {
    processes: [Option<Process>; PROCESSES_MAX],
    current_index: usize,
    next_pid: usize,
}

impl ProcessManager {
    pub fn new() -> Self {
        let mut processes = core::array::from_fn(|_| None);
        processes[0] = Some(Process::new_idle());

        Self {
            processes,
            current_index: 0,
            next_pid: 1,
        }
    }

    fn create_process(&mut self, entry: extern "C" fn() -> isize) -> Option<usize> {
        let index = self.processes.iter().position(Option::is_none)?;
        let pid = self.next_pid;
        let next_pid = self.next_pid.checked_add(1)?;
        let parent_pid = Some(self.current_pid());
        let process = Process::new(pid, parent_pid, entry)?;
        self.processes[index] = Some(process);
        self.next_pid = next_pid;
        Some(pid)
    }

    pub fn process_count(&self) -> usize {
        self.processes.iter().filter(|slot| slot.is_some()).count()
    }

    pub fn runnable_process_count(&self) -> usize {
        self.processes
            .iter()
            .flatten()
            .filter(|process| process.state == ProcessState::Runnable)
            .count()
    }

    pub fn current_pid(&self) -> usize {
        self.processes[self.current_index]
            .as_ref()
            .map_or(0, |p| p.pid())
    }

    fn next_runnable_index(&self) -> usize {
        let process_count = self.processes.len();

        for offset in 1..=process_count {
            let index = (self.current_index + offset) % process_count;

            if self.processes[index]
                .as_ref()
                .map_or(false, |p| p.state() == ProcessState::Runnable)
            {
                return index;
            }
        }

        0
    }

    fn prepare_yield(&mut self) -> Option<SwitchContext> {
        let previous = self.current_index;
        let next = self.next_runnable_index();

        if previous == next {
            return None;
        }

        let processes = self.processes.as_mut_ptr();

        let previous_process = unsafe {
            (*processes.add(previous))
                .as_mut()
                .expect("current process slot is empty")
        };

        let next_process = unsafe {
            (*processes.add(next))
                .as_ref()
                .expect("next process slot is empty")
        };

        // SAFETY:
        // - previous and next are valid indices in processes.
        let previous_sp_ptr = core::ptr::addr_of_mut!(previous_process.sp);
        let next_sp_ptr = core::ptr::addr_of!(next_process.sp);

        self.current_index = next;

        Some(SwitchContext {
            previous_sp_ptr,
            next_sp_ptr,
        })
    }

    fn release_exited_resources(&mut self) {
        for (index, slot) in self.processes.iter_mut().enumerate() {
            if index == self.current_index {
                continue;
            }

            let Some(process) = slot.as_mut() else {
                continue;
            };

            if matches!(process.state, ProcessState::Exited(_)) {
                process.stack = None;
                process.entry = None;
                process.sp = 0;
            }
        }
    }

    fn try_wait(&mut self, parent_pid: usize, child_pid: usize) -> Result<WaitResult> {
        let index = self
            .processes
            .iter()
            .position(|slot| {
                slot.as_ref()
                    .is_some_and(|process| process.pid == child_pid)
            })
            .ok_or(Error::NoSuchProcess)?;

        let child = self.processes[index]
            .as_ref()
            .expect("located process slot must not be empty");

        if child.parent_pid != Some(parent_pid) {
            return Err(Error::NotAChildProcess);
        }

        match child.state {
            ProcessState::Runnable => Ok(WaitResult::Running),

            ProcessState::Exited(code) => {
                self.processes[index] = None;
                Ok(WaitResult::Exited(code))
            }

            ProcessState::Idle => Err(Error::NotAChildProcess),
        }
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

static ROOT_PROCESS_MANAGER: Mutex<Option<ProcessManager>> = Mutex::new(None);

pub fn init() {
    let mut root = ROOT_PROCESS_MANAGER.lock();

    assert!(root.is_none(), "process manager is already initialized");

    *root = Some(ProcessManager::new());
}

pub fn create_process(entry: extern "C" fn() -> isize) -> Option<usize> {
    let mut root = ROOT_PROCESS_MANAGER.lock();
    let manager = root.as_mut()?;

    manager.create_process(entry)
}

fn release_exited_resources() {
    let mut root = ROOT_PROCESS_MANAGER.lock();
    let manager = root.as_mut().expect("process manager is not initialized");

    manager.release_exited_resources();
}

pub fn yield_process() {
    let switch_info = {
        let mut root = ROOT_PROCESS_MANAGER.lock();
        let manager = root.as_mut().expect("process manager is not initialized");
        manager.prepare_yield()
    };

    let Some(switch_info) = switch_info else {
        return;
    };

    // SAFETY:
    // - prepare_yield returned pointers to valid Process::sp fields.
    // - Processes are stored in a preallocated Vec and are not removed.
    // - The ProcessManager lock has been released before switching.
    unsafe {
        switch_context(switch_info.previous_sp_ptr, switch_info.next_sp_ptr);
    }

    release_exited_resources();
}

fn current_process_entry() -> ProcessEntry {
    let root = ROOT_PROCESS_MANAGER.lock();
    let manager = root.as_ref().expect("process manager is not initialized");
    let current = &manager.processes[manager.current_index];

    current
        .as_ref()
        .expect("current process is not initialized")
        .entry
        .expect("current process has no entry function")
}

extern "C" fn process_entry_trampoline() -> ! {
    release_exited_resources();

    let entry = current_process_entry();
    let exit_code = entry();

    exit_process(exit_code);
}

pub fn exit_process(exit_code: isize) -> ! {
    let switch_info = {
        let mut root = ROOT_PROCESS_MANAGER.lock();
        let manager = root.as_mut().expect("process manager is not initialized");
        // 現在ProcessをExited(exit_code)にする
        let current = &mut manager.processes[manager.current_index]
            .as_mut()
            .expect("current process slot is empty");
        current.state = ProcessState::Exited(exit_code);
        // 次のRunnable Processを選ぶ
        manager.prepare_yield()
    };

    // switch_contextする
    if let Some(switch_info) = switch_info {
        // SAFETY:
        // - prepare_yield returned pointers to valid Process::sp fields.
        // - Processes are stored in a preallocated Vec and are not removed.
        // - The ProcessManager lock has been released before switching.
        unsafe {
            switch_context(switch_info.previous_sp_ptr, switch_info.next_sp_ptr);
        }
    }

    panic!("exited process was resumed");
}

pub fn process_state(pid: usize) -> Option<ProcessState> {
    let root = ROOT_PROCESS_MANAGER.lock();
    let manager = root.as_ref()?;

    manager
        .processes
        .iter()
        .flatten()
        .find(|process| process.pid == pid)
        .map(|process| process.state)
}

pub fn try_wait_process(child_pid: usize) -> Result<WaitResult> {
    let mut root = ROOT_PROCESS_MANAGER.lock();
    let manager = root.as_mut().expect("process manager is not initialized");

    let parent_pid = manager.current_pid();
    manager.try_wait(parent_pid, child_pid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::context::CONTEXT_SIZE;

    extern "C" fn return_zero() -> isize {
        0
    }

    extern "C" fn return_ten() -> isize {
        10
    }

    #[test_case]
    fn idle_and_normal_processes_are_initialized() {
        // プロセスが正しく初期化されているかを確認するテスト

        let manager = ProcessManager::new();
        let idle = manager.processes[0].as_ref().expect("PID 0 must exist");

        assert_eq!(idle.pid(), 0);
        assert_eq!(idle.state(), ProcessState::Idle);
        assert_eq!(idle.stack_start(), None);
        assert_eq!(manager.current_pid(), 0);

        let process = Process::new(1, Some(0), return_zero).expect("failed to create process");

        let sp = process.stack_pointer();
        let stack_start = process
            .stack_start()
            .expect("normal process must have a kernel stack");
        let stack_end = process
            .stack_end()
            .expect("normal process must have a kernel stack");

        assert_eq!(process.pid(), 1);
        assert_eq!(process.state(), ProcessState::Runnable);
        assert_eq!(sp % 16, 0);
        assert!(stack_start <= sp);
        assert!(sp + CONTEXT_SIZE <= stack_end);
    }

    #[test_case]
    fn process_manager_schedules_round_robin() {
        // プロセスがラウンドロビンスケジューリングされることを確認するテスト

        let mut manager = ProcessManager::new();

        let pid1 = manager
            .create_process(return_zero)
            .expect("failed to create process 1");

        let pid2 = manager
            .create_process(return_zero)
            .expect("failed to create process 2");

        assert_eq!(pid1, 1);
        assert_eq!(pid2, 2);
        assert_eq!(manager.process_count(), 3);

        // PID 0 → PID 1
        assert_eq!(manager.next_runnable_index(), 1);

        // PID 1 → PID 2
        manager.current_index = 1;
        assert_eq!(manager.next_runnable_index(), 2);

        // PID 2 → PID 1
        manager.current_index = 2;
        assert_eq!(manager.next_runnable_index(), 1);
    }

    #[test_case]
    fn process_return_value_is_recorded() {
        // プロセスの終了コードが正しく記録されることを確認するテスト

        let mut root = ROOT_PROCESS_MANAGER.lock();
        *root = Some(ProcessManager::new());
        drop(root);

        let pid = create_process(return_ten).expect("failed to create process");

        assert_eq!(process_state(pid), Some(ProcessState::Runnable),);

        yield_process();

        assert_eq!(process_state(pid), Some(ProcessState::Exited(10)),);

        assert_eq!(process_state(usize::MAX), None);
    }
}
