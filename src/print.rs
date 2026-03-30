use core::fmt::{self, Write};

struct Console;

impl Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            crate::sbi::putchar(b);
        }
        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) {
    let _ = Console.write_fmt(args);
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::print::_print(core::format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    ($fmt:expr) => {
        $crate::print!(concat!($fmt, "\n"))
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::print!(concat!($fmt, "\n"), $($arg)*)
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ($crate::print!("\x1b[32m[INFO]\x1b[0m {}:{:<3}: {}\n", file!(), line!(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ($crate::print!("\x1b[33m[WARN]\x1b[0m {}:{:<3}: {}\n", file!(), line!(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ($crate::print!("\x1b[31m[ERROR]\x1b[0m {}:{:<3}: {}\n", file!(), line!(), format_args!($($arg)*)));
}
