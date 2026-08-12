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

const PROGRAM_DEMO: usize = 1;
const PROGRAM_YES: usize = 2;
const PROCESS_TERMINATE: usize = 0;
const PROCESS_STOP: usize = 1;
const PROCESS_CONTINUE: usize = 2;
const PROCESS_INTERRUPT: usize = 3;
const CTRL_C: u8 = 0x03;
const CTRL_Z: u8 = 0x1a;
const LINE_MAX: usize = 128;
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

fn read_line(buffer: &mut [u8; LINE_MAX]) -> usize {
    let mut length = 0;

    loop {
        let Some(ch) = getchar() else {
            yield_process();
            continue;
        };

        match ch {
            b'\r' | b'\n' => {
                print("\r\n");
                return length;
            }
            0x08 | 0x7f if length != 0 => {
                length -= 1;
                print("\x08 \x08");
            }
            0x20..=0x7e if length < buffer.len() => {
                buffer[length] = ch;
                length += 1;
                putchar(ch);
            }
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

fn foreground(pid: usize, last_job: &mut Option<usize>) {
    loop {
        match wait_process(pid) {
            WaitResult::Running => {
                match getchar() {
                    Some(CTRL_C) => {
                        print("^C\r\n");
                        if !control_process(pid, PROCESS_INTERRUPT) {
                            print("failed to interrupt process\r\n");
                            return;
                        }
                    }
                    Some(CTRL_Z) => {
                        print("^Z\r\n");
                        if control_process(pid, PROCESS_STOP) {
                            *last_job = Some(pid);
                            print("[");
                            print_number(pid);
                            print("] stopped\r\n");
                        } else {
                            print("failed to stop process\r\n");
                        }
                        return;
                    }
                    _ => {}
                }
                yield_process();
            }
            WaitResult::Stopped => {
                *last_job = Some(pid);
                print("[");
                print_number(pid);
                print("] stopped\r\n");
                return;
            }
            WaitResult::Exited(code) => {
                if *last_job == Some(pid) {
                    *last_job = None;
                }
                report_exit(pid, code);
                return;
            }
            WaitResult::Error => {
                print("not a child process\r\n");
                return;
            }
        }
    }
}

fn start_job(
    program: usize,
    argument: usize,
    name: &str,
    background: bool,
    last_job: &mut Option<usize>,
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
    *last_job = Some(pid);

    if !background {
        foreground(pid, last_job);
    }
}

fn command_wait(argument: &[u8]) {
    let Some(pid) = parse_number(argument) else {
        print("usage: wait <pid>\r\n");
        return;
    };

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
                report_exit(pid, code);
                return;
            }
            WaitResult::Error => {
                print("not a child process\r\n");
                return;
            }
        }
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

fn job_pid(argument: &[u8], last_job: Option<usize>, usage: &str) -> Option<usize> {
    let pid = if argument.is_empty() {
        last_job
    } else {
        parse_number(argument)
    };

    if pid.is_none() {
        print(usage);
        print("\r\n");
    }
    pid
}

fn command_fg(argument: &[u8], last_job: &mut Option<usize>) {
    let Some(pid) = job_pid(argument, *last_job, "usage: fg [pid]") else {
        return;
    };
    if !control_process(pid, PROCESS_CONTINUE) {
        print("failed to continue process\r\n");
        return;
    }
    foreground(pid, last_job);
}

fn command_bg(argument: &[u8], last_job: &mut Option<usize>) {
    let Some(pid) = job_pid(argument, *last_job, "usage: bg [pid]") else {
        return;
    };
    if control_process(pid, PROCESS_CONTINUE) {
        *last_job = Some(pid);
        print("[");
        print_number(pid);
        print("] running\r\n");
    } else {
        print("failed to continue process\r\n");
    }
}

fn command_kill(argument: &[u8], last_job: &mut Option<usize>) {
    let (control, pid_bytes) = if let Some(pid) = argument.strip_prefix(b"-STOP ") {
        (PROCESS_STOP, pid)
    } else if let Some(pid) = argument.strip_prefix(b"-CONT ") {
        (PROCESS_CONTINUE, pid)
    } else {
        (PROCESS_TERMINATE, argument)
    };
    let Some(pid) = parse_number(pid_bytes) else {
        print("usage: kill [-STOP|-CONT] <pid>\r\n");
        return;
    };

    if !control_process(pid, control) {
        print("failed to control process\r\n");
        return;
    }

    if control == PROCESS_STOP {
        *last_job = Some(pid);
        print("[");
        print_number(pid);
        print("] stopped\r\n");
    } else if control == PROCESS_CONTINUE {
        *last_job = Some(pid);
        print("[");
        print_number(pid);
        print("] running\r\n");
    } else {
        if *last_job == Some(pid) {
            *last_job = None;
        }
        let _ = wait_process(pid);
        print("[");
        print_number(pid);
        print("] terminated\r\n");
    }
}

fn execute_command(line: &[u8], last_job: &mut Option<usize>) {
    match line {
        b"" => {}
        b"help" => {
            print("echo <text>  ps  jobs  demo [&]  yes  wait <pid>\r\n");
            print("fg [pid]  bg [pid]  kill [-STOP|-CONT] <pid>  clear  poweroff\r\n");
            print("Ctrl-C interrupts and Ctrl-Z stops the foreground job\r\n");
        }
        b"ps" => command_ps(),
        b"jobs" => command_jobs(),
        b"demo" => start_job(PROGRAM_DEMO, 1, "demo", false, last_job),
        b"demo &" => start_job(PROGRAM_DEMO, 1, "demo", true, last_job),
        b"yes" => start_job(PROGRAM_YES, 0, "yes", false, last_job),
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
        _ if line.starts_with(b"wait ") => command_wait(&line[5..]),
        b"fg" => command_fg(b"", last_job),
        _ if line.starts_with(b"fg ") => command_fg(&line[3..], last_job),
        b"bg" => command_bg(b"", last_job),
        _ if line.starts_with(b"bg ") => command_bg(&line[3..], last_job),
        _ if line.starts_with(b"kill ") => command_kill(&line[5..], last_job),
        _ => print("unknown command\r\n"),
    }
}

fn shell() -> ! {
    print("\r\nvertos user shell (no-MMU)\r\n");
    print("type 'help' for commands\r\n");

    let mut line = [0u8; LINE_MAX];
    let mut last_job = None;
    loop {
        print("vertos> ");
        let length = read_line(&mut line);
        execute_command(&line[..length], &mut last_job);
    }
}

fn demo_worker(worker: usize) -> ! {
    for step in 0..4 {
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
        demo_worker(argument)
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
