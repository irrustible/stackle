//! X86-64 is pretty simple. We have loads of registers to play with and we only use 4 instructions.
//! It is made slightly less clear by some leaf function optimisations which may help performance on
//! older processors.
//!
//! Fun ABI facts:
//!
//! * `sp` ought to be aligned to 16 bytes when making a function call. This is poorly enforced, but
//!   if you don't it's liable to confuse some software and may decrease performance.
//! * There is a 128-byte red zone below the stack we can use for leaf function storage.
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
    "lea rax, [rip + 2f]", // calculate address of end of this function with forward ref
    "mov [rsp - 8],  rax", // save end of function as the return address
    "mov [rsp - 16], rbp", // save the frame pointer in case it is used (it probably isn't)
    "mov [rsp - 24], rbx", // save llvm's nefarious porpoises register.
    // our stack should now look like this (in the red zone):
    // | rsp rel | data           |
    // |---------|----------------|
    // | -8      | return address |
    // | -16     | frame pointer  |
    // | -24     | llvm obscurity |

    
    // step 2: set up the trampoline frame in the new stack
    "mov [rdx - 8],  rcx", // trampoline
    "mov [rdx - 16], rdi", // fun

    // the new stack should now look like this:
    // | end rel | data                |
    // |---------|---------------------|
    // | -8      | trampoline function |
    // | -16     | entrypoint function |

    // step 3: setting up parameters
    // rdi = fun (we haven't touched it)
    "mov rdi, rsp",        // current stack pointer -> arg 1

    // argument layout should now be:
    // | register | value                   |
    // |----------|-------------------------|
    // | rdi      | rsp (our stack pointer) |
    // | rsi      | arg (closure parameter) |
    
    // step 4: calling trampoline on the new stack.
    "xor rbx, rbx",        // zero out rbx (reset nefarious porpoise state)
    "xor rbp, rbp",        // zero out rbp (meaning "top of call chain")
    "lea rsp, [rdx - 16]", // set the correct stack pointer
    "jmp rcx",             // switch to trampoline
    
    // End of function, as taken in first instruction. register layout should now be:
    // | register | value                   |
    // |----------|-------------------------|
    // | rsi      | arg                     |
    // | rdx      | paused stack pointer    |

    "2:",
    inout("rdi") fun => _,
    inout("rsi") arg => _,
    inout("rdx") stack,
    inout("rcx") trampoline => _,
    clobber_abi("C")
  );
  stack
}

/// Pauses the current stack context and resumes another.
///
/// # Safety
///
/// Behaviour is undefined if:
/// * The stack was not paused correctly
#[inline(always)]
pub unsafe extern "C" fn switch(mut stack: *mut usize, mut arg: usize) -> Switch {
  asm!(
    // spill to stack
    "lea rax, [rip + 2f]", // calculate address of end of this function with forward ref
    "mov [rsp - 8],  rax", // save end of function as the return address
    "mov [rsp - 16], rbp", // save the frame pointer (we aren't allowed to clobber it)
    "mov [rsp - 24], rbx", // save llvm's nefarious porpoises register (no clobbering)
    // our stack should now look like this:
    // | rsp rel | data           |
    // |---------|----------------|
    // | -8      | return address |
    // | -16     | frame pointer  |
    // | -24     | llvm obscurity |

    // step 2: switch stacks
    "mov rdx, rsp",        // save current stack pointer into rdx
    "mov rsp, rdi",        // load new stack pointer from resume_stack

    // step 2: state restoration (inverse of preservation) and branching
    "mov rbx, [rdi - 24]",
    "mov rbp, [rdi - 16]",
    "mov rax, [rdi - 8]",
    "jmp rax",

    // our internal calling convention is this:
    // | register | value                   |
    // |----------|-------------------------|
    // | rdi      |                         |
    // | rsi      | arg                     |
    // | rdx      | paused stack pointer    |

    // the end of the function, always called into by resume()
    "2:", 
    inout("rdi") stack => _,
    inout("rsi") arg,
    out("rdx") stack,
    out("rcx") _,
    out("rax") _,
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
  ".cfi_undefined rip",    // stop unwinding at this frame
  ".cfi_undefined rsp",    // stop the call chain at this frame (for gdb)
  "call [rsp]",            // call the function in a new stack frame.
  ".cfi_endproc"           // function epilogue
);
