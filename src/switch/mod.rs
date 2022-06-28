mod arch;
pub use arch::*;

use core::mem::ManuallyDrop;

pub type InitFn =  unsafe extern "C" fn(*mut usize, *const u8);

#[repr(C)]
pub struct Switch {
  pub stack: *mut usize,
  pub arg:   usize,
}

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
pub unsafe fn link_closure_detached<F>(stack: *mut usize, closure: F) -> *mut usize
where F: FnOnce(*mut usize, usize) {
  let f = ManuallyDrop::new(closure);
  let f = (&f as *const ManuallyDrop<F>).cast::<u8>() as usize;
  link_detached(bootstrap_closure::<F>, f, stack)
}

unsafe extern "C" fn bootstrap_closure<F>(stack: *mut usize, closure: *const u8)
where F: FnOnce(*mut usize, usize) {
  let f = closure.cast::<F>().read();
  let switch = switch(stack, 0);
  f(switch.stack, switch.arg);
}
