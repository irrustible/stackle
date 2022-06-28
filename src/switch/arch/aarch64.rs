//! 
use crate::switch::{InitFn, Switch};
use core::arch::asm;

/// Links a coroutine with a new stack, starting a new call stack at a trampoline.
///
/// # Safety
///
/// * `stack` must be a properly aligned pointer to the end of a region.
/// * `stack` must either have a guard page allocated or not overflow.
#[inline(always)]
pub unsafe extern "C" fn link_detached(
  fun: InitFn,           // the function that the trampoline will call
  arg: usize,            // probably a pointer to a closure
  mut stack: *mut usize, // the end of a stack region.
) -> *mut usize {
  asm!(
    // step 1: state preservation. we must spill our state to the stack so we may be resumed.
    // adr = generate pc-relative address, 2f = forward reference to label 2.
    "adr lr, 2f",             // set the link register to the end of this function.
    // stp = store pair of registers, [sp, #-16] = predecrement sp by 16. implies sp -= 16
    "stp fp, lr, [sp, #-16]", // push the frame pointer and return address to the stack
    // our stack should now look like this:
    // | new sp rel | old sp rel | data           |
    // |------------|------------|----------------|
    // | +8         | -8         | return address |
    // | 0          | -16        | frame pointer  |

    // step 2: set up the trampoline frame in the new stack
    "stp x0, x3, [x2, #-16]" // push fun and trampoline to the stack. implies x2 -= 16
    // the new stack should now look like this:
    // | new x2 rel | old x2 rel | data                |
    // |------------|------------|---------------------|
    // | +8         | -8         | trampoline function |
    // | 0          | -16        | entrypoint function |

    // step 3: setting up parameters
    "mov sp, x0", // current stack pointer -> arg 1, overwriting 'fun'.
    // argument layout should now be:
    // | register | value                  |
    // |----------|------------------------|
    // | x0       | paused stack pointer   |
    // | x1       | arg (untouched)        |
    
    // step 4: calling trampoline on the new stack.
    "mov xzr, lr", // zero out the link register (meaning "top of call chain")
    "mov x2, sp",  // set the correct stack pointer
    "bx x3",       // switch to trampoline
    
    // End of function, as taken in first instruction. register layout should now be:
    // | register | value                   |
    // |----------|-------------------------|
    // | x1       | arg                     |
    // | x2       | paused stack pointer    |

    "2:",
    inout("x0") fun => _,
    inout("x1") arg => _,
    inout("x2") stack,
    inout("x3") trampoline => _,
    clobber_abi("C")
  );
  stack
}

/// Pauses the current stack context and resumes another.
///
/// If the pointer points to an `extern "C"` function then 
///
/// # Safety
///
/// Behaviour is undefined if:
/// * The stack was not paused correctly
#[inline(always)]
pub unsafe extern "C" fn switch(mut stack: *mut usize, mut arg: usize) -> Switch {
  asm!(
    // step 1: state preservation. we must spill our state to the stack so we may be resumed.
    // adr = generate pc-relative address, 2f = forward reference to label 2.
    "adr lr, 2f",             // set the link register to the end of this function.
    // stp = store pair of registers, [sp, #-16] = predecrement sp by 16. implies sp -= 16
    "stp fp, lr, [sp, #-16]", // push the frame pointer and return address to the stack
    // our stack should now look like this:
    // | new sp rel | old sp rel | data           |
    // |------------|------------|----------------|
    // | +8         | -8         | return address |
    // | 0          | -16        | frame pointer  |

    // step 2: set parameters stacks
    "mov sp, x2", // save current stack pointer into x2
    // argument layout should now be:
    // | register | value                |
    // |----------|----------------------|
    // | x1       | arg (untouched)      |
    // | x2       | paused stack pointer |

    // step 3: state restoration (inverse of preservation) and branching
    // ldp = load pair of registers, #16 = postincrement sp by 16, implies sp += 16
    "ldp fp, lr, [x0], #16" // load the frame pointer and return address from the stack
    "mov x0, sp",           // load new stack pointer
    "bx lr",                // branch to the return address.

    // End of function, as taken in first instruction. register layout should now be:
    // | register | value                   |
    // |----------|-------------------------|
    // | x1       | arg                     |
    // | x2       | paused stack pointer    |
    "2:", 
    inout("x0") stack => _,
    inout("x1") arg,
    out("x2") stack,
    clobber_abi("C")
  );
  Switch { stack, arg }
}

/* Trampoline function (terminates the call chain, becoming the first frame):
 * - called with an artificial frame.
 * - calls the function in a new frame.
 * - expects that function never to return.
 */
extern "C" {
    fn trampoline();
}

core::arch::global_asm!(
  ".global trampoline",
  ".align 16",             // put it at the start of a quadword to increase fetch perf.
  "trampoline:",
  ".cfi_startproc simple", // function prologue
  ".cfi_undefined lr",     // stop unwinding at this frame
  ".cfi_undefined fp",     // stop the call chain at this frame (for gdb)
  "bl sp",                 // call the function in a new stack frame.
  ".cfi_endproc"           // function epilogue
);
