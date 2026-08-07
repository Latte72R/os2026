//! Simple thread-safe mutex
//!
//! As the doc of SyncUnsafeCell says,
//! SyncUnsafeCell::get() can be used to get
//! *mut T from &SyncUnsafeCell<T> but we must
//! ensure that the access to the object pointed
//! is unique before dereferencing it.
//!
//! This mutex protects the data with AtomicBool
//! to ensure that the access to the contents
//! is unique so taking a mutable reference
//! to it will be safe.

use crate::error::Result;
use core::cell::SyncUnsafeCell;
use core::ops::Deref;
use core::ops::DerefMut;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
    data: &'a mut T,
}
impl<'a, T> MutexGuard<'a, T> {
    unsafe fn new(mutex: &'a Mutex<T>, data: &SyncUnsafeCell<T>) -> Self {
        Self {
            mutex,
            data: unsafe { &mut *data.get() },
        }
    }
}
unsafe impl<'a, T> Sync for MutexGuard<'a, T> {}
impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}
impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}
impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.is_taken.store(false, Ordering::Release)
    }
}

pub struct Mutex<T> {
    data: SyncUnsafeCell<T>,
    is_taken: AtomicBool,
}
impl<T: Sized> Mutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            data: SyncUnsafeCell::new(data),
            is_taken: AtomicBool::new(false),
        }
    }
    pub fn try_lock(&self) -> Result<MutexGuard<'_, T>> {
        if self
            .is_taken
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Ok(unsafe { MutexGuard::new(self, &self.data) })
        } else {
            Err(crate::error::Error::LockFailed)
        }
    }
    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.try_lock().expect("unimplemented!")
    }
}
// SAFETY: is_taken ensures exclusive access to data.
// T must be Send because locking may transfer access between execution contexts.
unsafe impl<T> Sync for Mutex<T> {}

#[test_case]
fn mutex_can_modify_value() {
    let mutex = Mutex::new(10);

    {
        let mut value = mutex.lock();
        *value += 5;
    }

    assert_eq!(*mutex.lock(), 15);
}

#[test_case]
fn mutex_prevents_double_lock() {
    let mutex = Mutex::new(10);

    let first = mutex.lock();

    // この時点では、2回目のtry_lockは失敗するはず
    assert!(mutex.try_lock().is_err());

    drop(first);

    // Guardを破棄したので、再び取得できるはず
    assert!(mutex.try_lock().is_ok());
}
