use crate::arch::trap::TrapFrame;
use crate::process::{self, ProcessControl, ProcessState, WaitResult};

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
const EXIT_SIGTERM: isize = 143;
const EXIT_SIGINT: isize = 130;
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
            let argument = match frame.a0 {
                PROGRAM_DEMO => Some(frame.a1),
                PROGRAM_YES => Some(usize::MAX),
                _ => None,
            };
            frame.a0 = argument
                .and_then(process::create_user_process)
                .unwrap_or(ERROR);
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
            Ok(WaitResult::Stopped) => {
                frame.a0 = 2;
                frame.a1 = 0;
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
                    ProcessState::Stopped => 3,
                };
            } else {
                frame.a0 = ERROR;
            }
        }
        SYS_SHUTDOWN => crate::platform::shutdown(true),
        SYS_PROCESS_CONTROL => {
            let control = match frame.a1 {
                PROCESS_TERMINATE => Some(ProcessControl::Terminate(EXIT_SIGTERM)),
                PROCESS_STOP => Some(ProcessControl::Stop),
                PROCESS_CONTINUE => Some(ProcessControl::Continue),
                PROCESS_INTERRUPT => Some(ProcessControl::Terminate(EXIT_SIGINT)),
                _ => None,
            };
            frame.a0 = control
                .and_then(|control| process::control_process(frame.a0, control).ok())
                .map_or(ERROR, |()| 0);
        }
        _ => frame.a0 = ERROR,
    }
}
