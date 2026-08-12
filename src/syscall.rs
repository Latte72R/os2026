use crate::arch::trap::TrapFrame;
use crate::process::{self, ProcessState, WaitResult};

const SYS_PUTCHAR: usize = 0;
const SYS_GETCHAR: usize = 1;
const SYS_YIELD: usize = 2;
const SYS_EXIT: usize = 3;
const SYS_SPAWN: usize = 4;
const SYS_WAIT: usize = 5;
const SYS_PROC_INFO: usize = 6;
const SYS_SHUTDOWN: usize = 7;

const PROGRAM_DEMO: usize = 1;
const ERROR: usize = usize::MAX;

pub fn handle(frame: &mut TrapFrame) {
    match frame.a7 {
        SYS_PUTCHAR => {
            crate::platform::putchar(frame.a0 as u8);
            frame.a0 = 0;
        }
        SYS_GETCHAR => {
            frame.a0 = crate::platform::getchar().map_or(ERROR, usize::from);
        }
        SYS_YIELD => {
            process::yield_process();
            frame.a0 = 0;
        }
        SYS_EXIT => process::exit_process(frame.a0 as isize),
        SYS_SPAWN => {
            frame.a0 = if frame.a0 == PROGRAM_DEMO {
                process::create_user_process(frame.a1).unwrap_or(ERROR)
            } else {
                ERROR
            };
        }
        SYS_WAIT => match process::try_wait_process(frame.a0) {
            Ok(WaitResult::Running) => {
                frame.a0 = 0;
                frame.a1 = 0;
            }
            Ok(WaitResult::Exited(code)) => {
                frame.a0 = 1;
                frame.a1 = code as usize;
            }
            Err(_) => {
                frame.a0 = ERROR;
                frame.a1 = 0;
            }
        },
        SYS_PROC_INFO => {
            if let Some(info) = process::process_info(frame.a0) {
                frame.a0 = 0;
                frame.a1 = info.pid;
                frame.a2 = info.parent_pid.unwrap_or(ERROR);
                frame.a3 = match info.state {
                    ProcessState::Idle => 0,
                    ProcessState::Runnable => 1,
                    ProcessState::Exited(_) => 2,
                };
            } else {
                frame.a0 = ERROR;
            }
        }
        SYS_SHUTDOWN => crate::platform::shutdown(true),
        _ => frame.a0 = ERROR,
    }
}
