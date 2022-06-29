//! Fun ABI facts:
//!
//! * `sp` must always be 16-byte aligned.
//! * No red zone under the stack pointer.
//! * Too many callee-push registers, what were they thinking?
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
    // addi = add immediate
    "addi sp, sp, -16",        // sp = sp - 16 (reserve space on the stack)
    // auipc - add upper immediate to program counter, i.e. make a pc-rel addr absolute.
    "auipc ra, %pcrel_lo(2f)", // ra = endofthisfunction
    // sw = store word (32 bit)
    "sw ra, 4(sp)",            // *(sp+4) = ra (save return address)
    "sw fp, 0(sp)",            // *sp = fp (save frame pointer)
    // our stack should now look like this:
    // | new sp rel | old sp rel | data           |
    // |------------|------------|----------------|
    // | +8         | -8         | return address |
    // | 0          | -16        | frame pointer  |

    "sw a3, -4(a2)", // *(a2-8) = a3 (store trampoline)
    "sw a0, -8(a2)", // *(a2-16) = a0 (push fun)
    // the new stack should now look like this:
    // | a2 rel | data                |
    // |--------|---------------------|
    // | -8     | trampoline function |
    // | -16    | entrypoint function |

    // step 3: setting up parameters
    // mv = move (actually a shortcut for `addi r10, sp, 0`)
    "mv a0, sp", // a0 = sp (current stack pointer -> arg 1, overwriting 'fun').
    // argument layout should now be:
    // | register | value                  |
    // |----------|------------------------|
    // | a0       | paused stack pointer   |
    // | a1       | arg (untouched)        |
    
    // step 4: calling trampoline on the new stack.
    // these are both ways of terminating the call chain
    "mv   zero, ra",      // ra = 0 (no return address)
    "mv   zero, fp",      // fp = 0 (no frame pointer)
    "addi sp,   a2, -16", // sp = a2 - 16 (set the correct stack pointer0)
    // j = jump (actually a shorthand for `jalr zero, a3`)
    "j    a3",            // transfer to trampoline
    
    // End of function, as taken in first instruction. register layout should now be:
    // | register | value                   |
    // |----------|-------------------------|
    // | a1       | arg                     |
    // | a2       | paused stack pointer    |

    "2:",
    inout("a0") fun => _,
    inout("a1") arg => _,
    inout("a2") stack,
    inout("a3") trampoline => _,
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
    "addi  sp,     sp, -16",   // sp = sp - 16 (reserve space on the stack)
    "auipc ra, %pcrel_lo(2f)", // ra = endofthisfunction
    "sw    ra,   4(sp)",       // *(sp+4) = ra (save return address)
    "sw    fp,   0(sp)",       // *sp = fp (save frame pointer)
    // our stack should now look like this:
    // | new sp rel | old sp rel | data           |
    // |------------|------------|----------------|
    // | +4         | -12        | return address |
    // | 0          | -16        | frame pointer  |

    // step 2: set arguments
    "mv sp, a2", // a2 = sp (current stack pointer)
    // argument layout should now be:
    // | register | value                |
    // |----------|----------------------|
    // | a1       | arg (untouched)      |
    // | a2       | paused stack pointer |

    // step 3: state restoration (inverse of preservation) and branching
    // lw = load word (32 bit)
    "lw   fp, 0(a0)",    // fp = *a0 (load the frame pointer)
    "lw   ra, 4(a0)",    // ra = *(a0 + 8) (load the return address)
    "addi sp,   a0, 16", // sp = r10 + 16 (set new sp but release the frame)
    "j    ra",           // transfer control back to the return address

    // End of function, as taken in first instruction. register layout should now be:
    // | register | value                   |
    // |----------|-------------------------|
    // | a1       | arg                     |
    // | a2       | paused stack pointer    |
    "2:", 
    inout("a0") stack => _,
    inout("a1") arg,
    out("a2")   stack,
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
  ".cfi_undefined ra",     // stop unwinding at this frame
  ".cfi_undefined fp",     // stop the call chain at this frame (for gdb)
  "call 8(sp)",            // call the function in a new stack frame.
  ".cfi_endproc"           // function epilogue
);
