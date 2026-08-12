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

const PROGRAM_DEMO: usize = 1;
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

fn spawn_demo(worker: usize) -> isize {
    syscall(SYS_SPAWN, PROGRAM_DEMO, worker)
}

enum WaitResult {
    Running,
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
            _ => "?",
        });
        print("\r\n");
    }
}

fn command_run_demo() {
    let first = spawn_demo(1);
    let second = spawn_demo(2);
    if first < 0 || second < 0 {
        print("failed to spawn demo\r\n");
        return;
    }

    print("started demo workers PID ");
    print_number(first as usize);
    print(" and ");
    print_number(second as usize);
    print("\r\n");
}

fn command_wait(argument: &[u8]) {
    let Some(pid) = parse_number(argument) else {
        print("usage: wait <pid>\r\n");
        return;
    };

    loop {
        match wait_process(pid) {
            WaitResult::Running => yield_process(),
            WaitResult::Exited(code) => {
                print("PID ");
                print_number(pid);
                print(" exited with ");
                if code < 0 {
                    putchar(b'-');
                    print_number(code.unsigned_abs());
                } else {
                    print_number(code as usize);
                }
                print("\r\n");
                return;
            }
            WaitResult::Error => {
                print("not a child process\r\n");
                return;
            }
        }
    }
}

fn execute_command(line: &[u8]) {
    match line {
        b"" => {}
        b"help" => print("help | echo <text> | ps | run demo | wait <pid> | clear | shutdown\r\n"),
        b"ps" => command_ps(),
        b"run demo" => command_run_demo(),
        b"clear" => print("\x1b[2J\x1b[H"),
        b"shutdown" => {
            let _ = syscall(SYS_SHUTDOWN, 0, 0);
        }
        _ if line.starts_with(b"echo ") => {
            for byte in &line[5..] {
                putchar(*byte);
            }
            print("\r\n");
        }
        _ if line.starts_with(b"wait ") => command_wait(&line[5..]),
        _ => print("unknown command\r\n"),
    }
}

fn shell() -> ! {
    print("\r\nvertos user shell (no-MMU)\r\n");
    print("type 'help' for commands\r\n");

    let mut line = [0u8; LINE_MAX];
    loop {
        print("vertos> ");
        let length = read_line(&mut line);
        execute_command(&line[..length]);
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

#[unsafe(no_mangle)]
extern "C" fn main(argument: usize) -> ! {
    if argument == 0 {
        shell()
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
