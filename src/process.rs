extern crate alloc;
use crate::arch::context::{initialize_stack, switch_context};
use crate::memory::Pages;
use crate::mutex::Mutex;
use alloc::vec::Vec;

const KERNEL_STACK_PAGES: usize = 2;
const PROCESSES_MAX: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Idle,
    Runnable,
}

#[derive(Debug)]
pub struct Process {
    pid: usize,
    state: ProcessState,
    sp: usize,
    stack: Option<Pages>,
}

impl Process {
    pub fn new(pid: usize, entry: extern "C" fn() -> !) -> Option<Self> {
        let mut stack = Pages::alloc(KERNEL_STACK_PAGES)?;
        let sp = initialize_stack(&mut stack, entry)?;

        Some(Self {
            pid,
            state: ProcessState::Runnable,
            sp,
            stack: Some(stack),
        })
    }

    fn new_idle() -> Self {
        Self {
            pid: 0,
            state: ProcessState::Idle,
            sp: 0,
            stack: None,
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

#[derive(Debug)]
pub struct ProcessManager {
    processes: Vec<Process>,
    current: usize,
}

impl ProcessManager {
    pub fn new() -> Self {
        let mut processes = Vec::with_capacity(PROCESSES_MAX);
        processes.push(Process::new_idle());

        Self {
            processes,
            current: 0,
        }
    }

    fn create_process(&mut self, entry: extern "C" fn() -> !) -> Option<usize> {
        if self.processes.len() >= PROCESSES_MAX {
            return None;
        }

        let pid = self.processes.len();
        let process = Process::new(pid, entry)?;

        self.processes.push(process);

        Some(pid)
    }

    pub fn process_count(&self) -> usize {
        self.processes.len()
    }

    pub fn current_pid(&self) -> usize {
        self.processes[self.current].pid()
    }

    fn next_runnable_index(&self) -> usize {
        let process_count = self.processes.len();

        for offset in 1..=process_count {
            let index = (self.current + offset) % process_count;

            if self.processes[index].state() == ProcessState::Runnable {
                return index;
            }
        }

        0
    }

    fn prepare_yield(&mut self) -> Option<SwitchContext> {
        let previous = self.current;
        let next = self.next_runnable_index();

        if previous == next {
            return None;
        }

        let processes = self.processes.as_mut_ptr();

        // SAFETY:
        // - previous and next are valid indices in processes.
        // - The Vec has capacity for PROCESSES_MAX entries.
        // - Processes are never removed or moved while scheduling.
        // 同じVecから一方を可変参照，もう一方を共有参照として同時に借用する
        let previous_sp_ptr = unsafe { core::ptr::addr_of_mut!((*processes.add(previous)).sp) };
        let next_sp_ptr = unsafe { core::ptr::addr_of!((*processes.add(next)).sp) };

        self.current = next;

        Some(SwitchContext {
            previous_sp_ptr,
            next_sp_ptr,
        })
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

pub fn create_process(entry: extern "C" fn() -> !) -> Option<usize> {
    let mut root = ROOT_PROCESS_MANAGER.lock();
    let manager = root.as_mut()?;

    manager.create_process(entry)
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
}

#[cfg(test)]
mod tests {
    use crate::arch::context::CONTEXT_SIZE;
    use crate::process::{Process, ProcessManager, ProcessState};

    extern "C" fn test_entry() -> ! {
        loop {
            core::hint::spin_loop();
        }
    }

    #[test_case]
    fn process_has_initialized_kernel_stack() {
        let process = Process::new(1, test_entry).expect("failed to create process");

        let sp = process.stack_pointer();

        assert_eq!(process.pid(), 1);
        assert_eq!(process.state(), ProcessState::Runnable);
        assert_eq!(sp % 16, 0);
        let stack_start = process
            .stack_start()
            .expect("normal process must have a kernel stack");

        let stack_end = process
            .stack_end()
            .expect("normal process must have a kernel stack");
        assert!(stack_start <= sp);
        assert!(sp + CONTEXT_SIZE <= stack_end);
    }

    #[test_case]
    fn rust_main_is_idle_process() {
        let manager = ProcessManager::new();

        assert_eq!(manager.current_pid(), 0);
    }

    #[test_case]
    fn process_manager_creates_processes() {
        let mut manager = ProcessManager::new();

        let pid1 = manager
            .create_process(test_entry)
            .expect("failed to create process 1");

        let pid2 = manager
            .create_process(test_entry)
            .expect("failed to create process 2");

        assert_eq!(pid1, 1);
        assert_eq!(pid2, 2);
        assert_eq!(manager.process_count(), 3);
    }

    #[test_case]
    fn process_manager_selects_processes_round_robin() {
        let mut manager = ProcessManager::new();

        manager
            .create_process(test_entry)
            .expect("failed to create process 1");

        manager
            .create_process(test_entry)
            .expect("failed to create process 2");

        // PID 0から最初に選ばれるのはPID 1
        assert_eq!(manager.current_pid(), 0);
        assert_eq!(manager.next_runnable_index(), 1);

        // PID 1の次はPID 2
        manager.current = 1;
        assert_eq!(manager.current_pid(), 1);
        assert_eq!(manager.next_runnable_index(), 2);

        // PID 2の次はPID 1
        manager.current = 2;
        assert_eq!(manager.current_pid(), 2);
        assert_eq!(manager.next_runnable_index(), 1);
    }
}
