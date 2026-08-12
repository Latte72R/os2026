#![no_std]
#![no_main]

use core::arch::{asm, global_asm};
use core::panic::PanicInfo;

global_asm!(include_str!("../boot/entry.S"));

const SYS_PUTCHAR: usize = 0;
const SYS_GETCHAR: usize = 1;
const SYS_YIELD: usize = 2;
const SYS_EXIT: usize = 3;
const SYS_SPAWN: usize = 4;
const SYS_WAIT: usize = 5;
const SYS_PROC_INFO: usize = 6;
const SYS_SHUTDOWN: usize = 7;
const SYS_PROCESS_CONTROL: usize = 8;

const PROGRAM_WORKERS: usize = 1;
const PROGRAM_YES: usize = 2;
const PROCESS_TERMINATE: usize = 0;
const PROCESS_STOP: usize = 1;
const PROCESS_CONTINUE: usize = 2;
const PROCESS_INTERRUPT: usize = 3;
const PROCESS_KILL: usize = 4;
const CTRL_C: u8 = 0x03;
const CTRL_Z: u8 = 0x1a;
const LINE_MAX: usize = 128;
const HISTORY_MAX: usize = 8;
const PROCESS_SLOTS: usize = 8;

fn syscall(number: usize, arg0: usize, arg1: usize) -> isize {
    let result: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") arg0 as isize => result,
            in("a1") arg1,
            in("a7") number,
        );
    }
    result
}

fn putchar(ch: u8) {
    let _ = syscall(SYS_PUTCHAR, ch as usize, 0);
}

fn print(text: &str) {
    for byte in text.bytes() {
        putchar(byte);
    }
}

fn print_number(mut value: usize) {
    let mut digits = [0u8; 20];
    let mut length = 0;

    if value == 0 {
        putchar(b'0');
        return;
    }

    while value != 0 {
        digits[length] = b'0' + (value % 10) as u8;
        length += 1;
        value /= 10;
    }

    for digit in digits[..length].iter().rev() {
        putchar(*digit);
    }
}

fn yield_process() {
    let _ = syscall(SYS_YIELD, 0, 0);
}

fn getchar() -> Option<u8> {
    let value = syscall(SYS_GETCHAR, 0, 0);
    (value >= 0).then_some(value as u8)
}

fn spawn_process(program: usize, argument: usize) -> isize {
    syscall(SYS_SPAWN, program, argument)
}

fn control_process(pid: usize, control: usize) -> bool {
    syscall(SYS_PROCESS_CONTROL, pid, control) == 0
}

enum WaitResult {
    Running,
    Stopped,
    Exited(isize),
    Error,
}

fn wait_process(pid: usize) -> WaitResult {
    let status: isize;
    let exit_code: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") pid as isize => status,
            lateout("a1") exit_code,
            in("a7") SYS_WAIT,
        );
    }

    match status {
        0 => WaitResult::Running,
        1 => WaitResult::Exited(exit_code),
        2 => WaitResult::Stopped,
        _ => WaitResult::Error,
    }
}

struct ProcessInfo {
    pid: usize,
    parent: isize,
    state: usize,
}

#[derive(Clone, Copy)]
struct Job {
    pids: [usize; 2],
    count: usize,
}

impl Job {
    fn single(pid: usize) -> Self {
        Self {
            pids: [pid, 0],
            count: 1,
        }
    }

    fn pair(first: usize, second: usize) -> Self {
        Self {
            pids: [first, second],
            count: 2,
        }
    }

    fn contains(self, pid: usize) -> bool {
        self.pids[..self.count].contains(&pid)
    }

    fn without(self, pid: usize) -> Option<Self> {
        if !self.contains(pid) {
            return Some(self);
        }

        if self.count == 1 {
            None
        } else {
            let remaining = if self.pids[0] == pid {
                self.pids[1]
            } else {
                self.pids[0]
            };
            Some(Self::single(remaining))
        }
    }
}

fn process_info(slot: usize) -> Option<ProcessInfo> {
    let status: isize;
    let pid: usize;
    let parent: isize;
    let state: usize;

    unsafe {
        asm!(
            "ecall",
            inlateout("a0") slot as isize => status,
            lateout("a1") pid,
            lateout("a2") parent,
            lateout("a3") state,
            in("a7") SYS_PROC_INFO,
        );
    }

    (status == 0).then_some(ProcessInfo { pid, parent, state })
}

fn parse_number(bytes: &[u8]) -> Option<usize> {
    if bytes.is_empty() {
        return None;
    }

    let mut value = 0usize;
    for byte in bytes {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add((byte - b'0') as usize)?;
    }
    Some(value)
}

struct CommandHistory {
    entries: [[u8; LINE_MAX]; HISTORY_MAX],
    lengths: [usize; HISTORY_MAX],
    count: usize,
}

impl CommandHistory {
    fn new() -> Self {
        Self {
            entries: [[0; LINE_MAX]; HISTORY_MAX],
            lengths: [0; HISTORY_MAX],
            count: 0,
        }
    }

    fn push(&mut self, command: &[u8]) {
        if command.is_empty()
            || (self.count != 0
                && self.entries[self.count - 1][..self.lengths[self.count - 1]] == *command)
        {
            return;
        }

        let index = if self.count < HISTORY_MAX {
            let index = self.count;
            self.count += 1;
            index
        } else {
            for index in 1..HISTORY_MAX {
                self.entries[index - 1] = self.entries[index];
                self.lengths[index - 1] = self.lengths[index];
            }
            HISTORY_MAX - 1
        };

        self.entries[index] = [0; LINE_MAX];
        self.entries[index][..command.len()].copy_from_slice(command);
        self.lengths[index] = command.len();
    }

    fn get(&self, index: usize) -> &[u8] {
        &self.entries[index][..self.lengths[index]]
    }
}

enum ReadLineResult {
    Line(usize),
    Cancelled,
}

fn wait_for_input() -> u8 {
    loop {
        if let Some(ch) = getchar() {
            return ch;
        }
        yield_process();
    }
}

fn move_cursor_left(count: usize) {
    for _ in 0..count {
        print("\x1b[D");
    }
}

fn move_cursor_right(count: usize) {
    for _ in 0..count {
        print("\x1b[C");
    }
}

fn replace_line(
    buffer: &mut [u8; LINE_MAX],
    length: &mut usize,
    cursor: &mut usize,
    replacement: &[u8],
) {
    let old_length = *length;
    move_cursor_left(*cursor);

    buffer[..replacement.len()].copy_from_slice(replacement);
    *length = replacement.len();
    *cursor = *length;

    for byte in replacement {
        putchar(*byte);
    }
    for _ in replacement.len()..old_length {
        putchar(b' ');
    }
    move_cursor_left(old_length.saturating_sub(replacement.len()));
}

fn read_line(buffer: &mut [u8; LINE_MAX], history: &CommandHistory) -> ReadLineResult {
    let mut length = 0;
    let mut cursor = 0;
    let mut history_index = history.count;
    let mut browsing_history = false;
    let mut draft = [0; LINE_MAX];
    let mut draft_length = 0;

    loop {
        let ch = wait_for_input();

        match ch {
            b'\r' | b'\n' => {
                print("\r\n");
                return ReadLineResult::Line(length);
            }
            CTRL_C => {
                move_cursor_right(length - cursor);
                print("^C\r\n");
                return ReadLineResult::Cancelled;
            }
            0x08 | 0x7f if cursor != 0 => {
                browsing_history = false;
                move_cursor_left(1);
                for index in cursor..length {
                    buffer[index - 1] = buffer[index];
                }
                length -= 1;
                cursor -= 1;
                for byte in &buffer[cursor..length] {
                    putchar(*byte);
                }
                putchar(b' ');
                move_cursor_left(length - cursor + 1);
            }
            0x20..=0x7e if length < buffer.len() => {
                browsing_history = false;
                for index in (cursor..length).rev() {
                    buffer[index + 1] = buffer[index];
                }
                buffer[cursor] = ch;
                length += 1;
                for byte in &buffer[cursor..length] {
                    putchar(*byte);
                }
                move_cursor_left(length - cursor - 1);
                cursor += 1;
            }
            0x1b if wait_for_input() == b'[' => match wait_for_input() {
                b'A' if history.count != 0 => {
                    if !browsing_history {
                        draft[..length].copy_from_slice(&buffer[..length]);
                        draft_length = length;
                        history_index = history.count;
                        browsing_history = true;
                    }
                    if history_index != 0 {
                        history_index -= 1;
                        replace_line(buffer, &mut length, &mut cursor, history.get(history_index));
                    }
                }
                b'B' if browsing_history => {
                    if history_index + 1 < history.count {
                        history_index += 1;
                        replace_line(buffer, &mut length, &mut cursor, history.get(history_index));
                    } else {
                        history_index = history.count;
                        browsing_history = false;
                        replace_line(buffer, &mut length, &mut cursor, &draft[..draft_length]);
                    }
                }
                b'C' if cursor < length => {
                    move_cursor_right(1);
                    cursor += 1;
                }
                b'D' if cursor != 0 => {
                    move_cursor_left(1);
                    cursor -= 1;
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn command_ps() {
    print("PID  PPID STATE\r\n");
    for slot in 0..PROCESS_SLOTS {
        let Some(info) = process_info(slot) else {
            continue;
        };

        print_number(info.pid);
        print("    ");
        if info.parent < 0 {
            print("-");
        } else {
            print_number(info.parent as usize);
        }
        print("    ");
        print(match info.state {
            0 => "idle",
            1 => "runnable",
            2 => "exited",
            3 => "stopped",
            _ => "?",
        });
        print("\r\n");
    }
}

fn report_exit(pid: usize, code: isize) {
    print("[");
    print_number(pid);
    match code {
        130 => print("] interrupted\r\n"),
        137 => print("] killed\r\n"),
        143 => print("] terminated\r\n"),
        _ => {
            print("] exited ");
            if code < 0 {
                putchar(b'-');
                print_number(code.unsigned_abs());
            } else {
                print_number(code as usize);
            }
            print("\r\n");
        }
    }
}

fn control_job(job: Job, active: &[bool; 2], control: usize) -> bool {
    let mut success = true;
    for (index, pid) in job.pids[..job.count].iter().enumerate() {
        if active[index] && !control_process(*pid, control) {
            success = false;
        }
    }
    success
}

fn remaining_job(job: Job, active: &[bool; 2]) -> Option<Job> {
    let mut pids = [0; 2];
    let mut count = 0;
    for (index, pid) in job.pids[..job.count].iter().enumerate() {
        if active[index] {
            pids[count] = *pid;
            count += 1;
        }
    }
    (count != 0).then_some(Job { pids, count })
}

fn print_job(job: Job) {
    print("[");
    for (index, pid) in job.pids[..job.count].iter().enumerate() {
        if index != 0 {
            print(",");
        }
        print_number(*pid);
    }
    print("]");
}

fn foreground(job: Job, last_job: &mut Option<Job>) {
    let mut active = [false; 2];
    active[..job.count].fill(true);

    loop {
        let mut all_stopped = true;
        for (index, pid) in job.pids[..job.count].iter().enumerate() {
            if !active[index] {
                continue;
            }

            match wait_process(*pid) {
                WaitResult::Running => all_stopped = false,
                WaitResult::Stopped => {}
                WaitResult::Exited(code) => {
                    active[index] = false;
                    report_exit(*pid, code);
                }
                WaitResult::Error => {
                    active[index] = false;
                    print("not a child process\r\n");
                }
            }
        }

        let Some(remaining) = remaining_job(job, &active) else {
            *last_job = None;
            return;
        };

        if all_stopped {
            *last_job = Some(remaining);
            print_job(remaining);
            print(" stopped\r\n");
            return;
        }

        match getchar() {
            Some(CTRL_C) => {
                print("^C\r\n");
                if !control_job(job, &active, PROCESS_INTERRUPT) {
                    print("failed to interrupt process\r\n");
                    return;
                }
            }
            Some(CTRL_Z) => {
                print("^Z\r\n");
                if control_job(job, &active, PROCESS_STOP) {
                    *last_job = Some(remaining);
                    print_job(remaining);
                    print(" stopped\r\n");
                } else {
                    print("failed to stop process\r\n");
                }
                return;
            }
            _ => {}
        }
        yield_process();
    }
}

fn start_job(
    program: usize,
    argument: usize,
    name: &str,
    background: bool,
    last_job: &mut Option<Job>,
) {
    let pid = spawn_process(program, argument);
    if pid < 0 {
        print("failed to spawn process\r\n");
        return;
    }

    let pid = pid as usize;
    print("[");
    print_number(pid);
    print("] running ");
    print(name);
    print("\r\n");
    let job = Job::single(pid);
    *last_job = Some(job);

    if !background {
        foreground(job, last_job);
    }
}

fn start_workers(background: bool, last_job: &mut Option<Job>) {
    let first = spawn_process(PROGRAM_WORKERS, 1);
    let second = spawn_process(PROGRAM_WORKERS, 2);
    if first < 0 || second < 0 {
        for pid in [first, second] {
            if pid >= 0 {
                let pid = pid as usize;
                let _ = control_process(pid, PROCESS_TERMINATE);
                let _ = wait_process(pid);
            }
        }
        print("failed to spawn workers\r\n");
        return;
    }

    let job = Job::pair(first as usize, second as usize);
    print_job(job);
    print(" running workers\r\n");
    *last_job = Some(job);

    if !background {
        foreground(job, last_job);
    }
}

fn wait_for_pid(pid: usize, last_job: &mut Option<Job>) {
    loop {
        match wait_process(pid) {
            WaitResult::Running => yield_process(),
            WaitResult::Stopped => {
                print("PID ");
                print_number(pid);
                print(" is stopped\r\n");
                return;
            }
            WaitResult::Exited(code) => {
                if let Some(job) = *last_job {
                    *last_job = job.without(pid);
                }
                report_exit(pid, code);
                return;
            }
            WaitResult::Error => {
                if let Some(job) = *last_job {
                    *last_job = job.without(pid);
                }
                print("not a child process\r\n");
                return;
            }
        }
    }
}

fn command_wait(argument: &[u8], last_job: &mut Option<Job>) {
    if !argument.is_empty() {
        let Some(pid) = parse_number(argument) else {
            print("usage: wait [pid]\r\n");
            return;
        };
        wait_for_pid(pid, last_job);
        return;
    }

    let mut children = [0; PROCESS_SLOTS];
    let mut count = 0;
    for slot in 0..PROCESS_SLOTS {
        let Some(info) = process_info(slot) else {
            continue;
        };
        if info.parent == 1 {
            children[count] = info.pid;
            count += 1;
        }
    }

    for pid in &children[..count] {
        wait_for_pid(*pid, last_job);
    }
}

fn command_jobs() {
    let mut found = false;
    for slot in 0..PROCESS_SLOTS {
        let Some(info) = process_info(slot) else {
            continue;
        };
        if info.parent != 1 {
            continue;
        }

        found = true;
        print("[");
        print_number(info.pid);
        print("] ");
        print(match info.state {
            1 => "running",
            2 => "exited",
            3 => "stopped",
            _ => "?",
        });
        print("\r\n");
    }
    if !found {
        print("no jobs\r\n");
    }
}

fn selected_job(argument: &[u8], last_job: Option<Job>, usage: &str) -> Option<Job> {
    let job = if argument.is_empty() {
        last_job
    } else {
        parse_number(argument).map(Job::single)
    };

    if job.is_none() {
        print(usage);
        print("\r\n");
    }
    job
}

fn command_fg(argument: &[u8], last_job: &mut Option<Job>) {
    let Some(job) = selected_job(argument, *last_job, "usage: fg [pid]") else {
        return;
    };
    let active = [true; 2];
    if !control_job(job, &active, PROCESS_CONTINUE) {
        print("failed to continue process\r\n");
        return;
    }
    print_job(job);
    print(" foreground\r\n");
    foreground(job, last_job);
}

fn command_bg(argument: &[u8], last_job: &mut Option<Job>) {
    let Some(job) = selected_job(argument, *last_job, "usage: bg [pid]") else {
        return;
    };
    let active = [true; 2];
    if control_job(job, &active, PROCESS_CONTINUE) {
        *last_job = Some(job);
        print_job(job);
        print(" running\r\n");
    } else {
        print("failed to continue process\r\n");
    }
}

fn command_kill(argument: &[u8], last_job: &mut Option<Job>) {
    let (control, pid_bytes) = if let Some(pid) = argument.strip_prefix(b"-STOP ") {
        (PROCESS_STOP, pid)
    } else if let Some(pid) = argument.strip_prefix(b"-CONT ") {
        (PROCESS_CONTINUE, pid)
    } else if let Some(pid) = argument
        .strip_prefix(b"-9 ")
        .or_else(|| argument.strip_prefix(b"-KILL "))
    {
        (PROCESS_KILL, pid)
    } else {
        (PROCESS_TERMINATE, argument)
    };
    let Some(pid) = parse_number(pid_bytes) else {
        print("usage: kill [-9|-KILL|-STOP|-CONT] <pid>\r\n");
        return;
    };

    if !control_process(pid, control) {
        print("failed to control process\r\n");
        return;
    }

    if control == PROCESS_STOP {
        *last_job = Some(Job::single(pid));
        print("[");
        print_number(pid);
        print("] stopped\r\n");
    } else if control == PROCESS_CONTINUE {
        *last_job = Some(Job::single(pid));
        print("[");
        print_number(pid);
        print("] running\r\n");
    } else {
        if let Some(job) = *last_job {
            *last_job = job.without(pid);
        }
        let _ = wait_process(pid);
        print("[");
        print_number(pid);
        if control == PROCESS_KILL {
            print("] killed\r\n");
        } else {
            print("] terminated\r\n");
        }
    }
}

fn execute_command(line: &[u8], last_job: &mut Option<Job>) {
    let (line, background) = if let Some(command) = line.strip_suffix(b" &") {
        (command, true)
    } else {
        (line, false)
    };

    match line {
        b"workers" => {
            start_workers(background, last_job);
            return;
        }
        b"yes" => {
            start_job(PROGRAM_YES, 0, "yes", background, last_job);
            return;
        }
        _ => {}
    }

    if background {
        print("cannot run shell built-in in background\r\n");
        return;
    }

    match line {
        b"" => {}
        b"help" => {
            print("echo <text>  ps  jobs  workers  yes  wait [pid]\r\n");
            print("fg [pid]  bg [pid]  kill [-9|-KILL|-STOP|-CONT] <pid>\r\n");
            print("append '&' to run an executable command in background\r\n");
            print("clear  poweroff\r\n");
            print("Ctrl-C cancels input or interrupts the foreground job\r\n");
            print("Ctrl-Z stops a job; arrow keys edit input and history\r\n");
        }
        b"ps" => command_ps(),
        b"jobs" => command_jobs(),
        b"clear" => print("\x1b[2J\x1b[H"),
        b"poweroff" | b"shutdown" => {
            let _ = syscall(SYS_SHUTDOWN, 0, 0);
        }
        _ if line.starts_with(b"echo ") => {
            for byte in &line[5..] {
                putchar(*byte);
            }
            print("\r\n");
        }
        b"wait" => command_wait(b"", last_job),
        _ if line.starts_with(b"wait ") => command_wait(&line[5..], last_job),
        b"fg" => command_fg(b"", last_job),
        _ if line.starts_with(b"fg ") => command_fg(&line[3..], last_job),
        b"bg" => command_bg(b"", last_job),
        _ if line.starts_with(b"bg ") => command_bg(&line[3..], last_job),
        b"kill" => print("usage: kill [-9|-KILL|-STOP|-CONT] <pid>\r\n"),
        _ if line.starts_with(b"kill ") => command_kill(&line[5..], last_job),
        _ => print("unknown command\r\n"),
    }
}

fn shell() -> ! {
    print("\r\nvertos user shell (no-MMU)\r\n");
    print("type 'help' for commands\r\n");

    let mut line = [0u8; LINE_MAX];
    let mut history = CommandHistory::new();
    let mut last_job = None;
    loop {
        print("vertos> ");
        match read_line(&mut line, &history) {
            ReadLineResult::Line(length) => {
                history.push(&line[..length]);
                execute_command(&line[..length], &mut last_job);
            }
            ReadLineResult::Cancelled => {}
        }
    }
}

fn worker_process(worker: usize) -> ! {
    for step in 0..10 {
        print("[worker ");
        print_number(worker);
        print("] step ");
        print_number(step);
        print("\r\n");
        yield_process();
    }

    let _ = syscall(SYS_EXIT, worker, 0);
    loop {
        core::hint::spin_loop();
    }
}

fn yes_worker() -> ! {
    loop {
        print("y\r\n");
        yield_process();
    }
}

#[unsafe(no_mangle)]
extern "C" fn main(argument: usize) -> ! {
    if argument == 0 {
        shell()
    } else if argument == usize::MAX {
        yes_worker()
    } else {
        worker_process(argument)
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    print("user panic\r\n");
    let _ = syscall(SYS_EXIT, usize::MAX, 0);
    loop {
        core::hint::spin_loop();
    }
}
