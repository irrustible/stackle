mod arch;
pub use arch::*;

use core::mem::ManuallyDrop;

pub type InitFn =  unsafe extern "C" fn(*const u8, *mut usize);

/// Moves the closure onto the new stack and calls it.
///
/// Closure receives the paused stack to return to as well as the first input (a usize).
///
/// # Safety
///
/// * Stack must be the end address of a properly aligned stack.
/// * One of:
///   * Stack must be allocated with a guard page OR
///   * Stack must never overflow (including red zone and signal space)
/// * Never return from the closure, escape it.
/// * Never unwind from the closure, catch any unwinding panic and escape.
pub unsafe fn link_closure<F>(closure: F, stack: *mut usize) -> *mut usize
where F: FnOnce(*mut usize, usize) {
  let f = ManuallyDrop::new(closure);
  let f = (&f as *const ManuallyDrop<F>).cast::<u8>();
  arch::link(f, bootstrap_closure::<F>, stack)
}

unsafe extern "C" fn bootstrap_closure<F>(fun: *const u8, stack: *mut usize)
where F: FnOnce(*mut usize, usize) {
  let f = fun.cast::<F>().read();
  let (new_stack, input) = arch::suspend(stack, 0);
  f(new_stack, input);
}
