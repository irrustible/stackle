use crate::switch::InitFn;
use core::arch::asm;

#[repr(C)]
pub struct Switch {
  pub stack: *mut usize,
  pub arg:   usize,
}

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

    // our stack should now look like this:
    // | rsp rel | data           |
    // |---------|----------------|
    // | -8      | return address |
    // | -16     | frame pointer  |
    // | -24     | llvm obscurity |

    // You may notice all of those are below the stack pointer. Isn't that naughty?
    //
    // No, the ABI for this platform provides a 128-byte "red zone" under the stack for leaf
    // functions to store their data.
    
    // step 2: set up the trampoline frame in the new stack
    "mov [rdx - 8],  rcx", // trampoline
    "mov [rdx - 16], rdi", // fun

    // the new stack should now look like this:
    // | end rel | data                |
    // |---------|---------------------|
    // | -8      | entrypoint function |
    // | -16     | caller frame addr   |

    // step 3: setting up parameters
    // rdi = fun (we haven't touched it)
    "mov rdi, rsp",        // current stack pointer -> arg 1

    // argument layout should now be:
    // | register | value                   |
    // |----------|-------------------------|
    // | rdi      | fun (closure parameter) |
    // | rsi      | rsp (our stack pointer) |
    
    // step 4: calling trampoline on the new stack.
    "xor rbx, rbx",        // zero out rbx
    "xor rbp, rbp",        // zero out rbp (meaning "top of call chain")
    "lea rsp, [rdx - 16]", // set the correct stack pointer
    "jmp rcx",             // switch to trampoline
    
    // End of function, as taken in first instruction. register layout should now be:
    // | register | value                   |
    // |----------|-------------------------|
    // | rdi      | our sp (ignored)        |
    // | rsi      | arg                     |
    // | rdx      | paused stack pointer    |

    "2:",
    inout("rdi") fun => _,
    inout("rsi") arg => _,
    inout("rdx") stack,
    inout("rcx") trampoline => _,
    out("rax") _,                 // scratch
    // clobber_abi("C")
    out("r8") _,    out("r9") _,    out("r10") _,   out("r11") _,
    out("r12") _,   out("r13") _,   out("r14") _,   out("r15") _,
    out("xmm0") _,  out("xmm1") _,  out("xmm2") _,  out("xmm3") _,
    out("xmm4") _,  out("xmm5") _,  out("xmm6") _,  out("xmm7") _,
    out("xmm8") _,  out("xmm9") _,  out("xmm10") _, out("xmm11") _,
    out("xmm12") _, out("xmm13") _, out("xmm14") _, out("xmm15") _,
  );
  stack
}

/// Pauses the current stack context and resumes another.
///
/// If the pointer points to an `extern "C"` function then the `arg` element is forwarded to it
/// through the `rdi` register.
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

    // switch stacks
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
    // clobber_abi("C")
    out("r8") _,    out("r9") _,    out("r10") _,   out("r11") _,
    out("r12") _,   out("r13") _,   out("r14") _,   out("r15") _,
    out("xmm0") _,  out("xmm1") _,  out("xmm2") _,  out("xmm3") _,
    out("xmm4") _,  out("xmm5") _,  out("xmm6") _,  out("xmm7") _,
    out("xmm8") _,  out("xmm9") _,  out("xmm10") _, out("xmm11") _,
    out("xmm12") _, out("xmm13") _, out("xmm14") _, out("xmm15") _,
  );
  Switch { stack, arg }
}

/* Trampoline function (terminates the call chain, becoming the first frame):
 * - called with an artificial frame.
 * - calls the function in the artificial frame.
 * - expects that function never to return.
 */

// On nightly, we can implement the trampoline with a naked fn
#[cfg(feature="nightly")]
#[naked]
unsafe extern "C" fn trampoline() {
  // .cfi_undefined marks a register as unrestorable in DWARF
  asm!(
    ".cfi_undefined rip", // This one's for the unwinder, to terminate the call chain.
    ".cfi_undefined rbp", // This one's for gdb, to avoid a "..." stack entry above us.
    "call [rsp]",         // call the function we left on the stack
    options(noreturn)
  )
}

// In stable rust, we can use global_asm and an extern fn
// Note: we haven't actually tested this at all.
#[cfg(not(feature="nightly"))]
extern "C" {
    fn trampoline();
}

#[cfg(not(feature="nightly"))]
core::arch::global_asm!(
  ".global trampoline",
  ".align 16",
  "trampoline:",
  ".cfi_startproc simple",
  ".cfi_undefined rip",
  ".cfi_undefined rbp",
  "call [rsp]",
  ".cfi_endproc"
);
