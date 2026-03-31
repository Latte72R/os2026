use core::any::type_name;

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        let test_name = type_name::<T>();
        crate::println!("[RUNNING] >>> {}", test_name);
        self();
        crate::println!("[PASS   ] <<< {}", test_name);
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    crate::println!("Running {} tests...", tests.len());
    for test in tests {
        test.run();
    }
    crate::println!("Completed {} tests!", tests.len());
    crate::sbi::shutdown(true);
}
