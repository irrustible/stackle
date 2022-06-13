use crate::switch::InitFn;
use core::arch::asm;

/// Links a coroutine with a stack.
///
/// 
///
/// # Safety
#[inline(always)]
pub unsafe fn link(
  fun:   *const u8,
  entry: InitFn,
  stack: *mut usize,
) -> *mut usize {
  let paused_stack: *mut usize;
  asm!(
    // step 1: state preservation. we must spill our state to the stack so we may be resumed.
    "lea rax, [rip + 1f]", // calculate address of end of this function with forward ref
    "mov [rsp - 8],  rax", // save end of function as the return address
    "mov [rsp - 16], rbp", // save the frame pointer
    "mov [rsp - 24], rbx", // save llvm's nefarious porpoises register.

    // step 2: setting up the first frame in the new stack
    "mov [rdx - 8],  rsi",  // entry function address
    "mov [rdx - 16], rsp",  // caller frame address (our stack pointer)

    // the stack should now look like this:
    // | end rel | data                |
    // |---------|---------------------|
    // | -8      | entrypoint function |
    // | -16     | caller frame addr   |

    // step 3: setting up parameters
    // rdi = fun (we haven't touched it)
    "mov rsi, rsp",        // current stack pointer -> arg 2

    // argument layout should now be:
    // | register | value                   |
    // |----------|-------------------------|
    // | rdi      | fun (closure parameter) |
    // | rsi      | rsp (our stack pointer) |
    
    // step 4: calling trampoline on the new stack.
    "mov rbx, rbx",        // zero out rbx
    "lea rbp, [rdx - 16]", // set the correct frame pointer
    "mov rsp, rbp", // set the correct stack pointer
    "jmp rcx",             // switch to trampoline
    
    // End of function, as taken in first instruction. register layout should now be:
    // | register | value                |
    // |----------|----------------------|
    // | rdx      | paused stack pointer |
    "1:",
    inout("rdi") fun => _,
    inout("rsi") entry => _,
    inout("rdx") stack => paused_stack,
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
  paused_stack
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
pub unsafe fn suspend(resume_stack: *mut usize, output: *const u8) -> (*mut usize, *mut u8, *const u8) {
  let paused_stack: *mut usize;
  let done: *mut u8;
  let input: *const u8;
  asm!(
    // spill to stack
    "lea rax, [rip + 1f]", // calculate address of end of this function with forward ref
    "mov [rsp - 8],  rax", // save end of function as the return address
    "mov [rsp - 16], rbp", // save the frame pointer
    "mov [rsp - 24], rbx", // save llvm's nefarious porpoises register.
    // resume from stack
    "mov rbx, [rdi - 24]",
    "mov rbp, [rdi - 16]",
    "mov rax, [rdi - 8]",

    "mov rdx, rsp",        // save current stack pointer into rdx

    // step 2: state restoration (inverse of preservation) and returning
    "mov rsp, rdi",


    "jmp rax",             // go there

    // | register | value                   |
    // |----------|-------------------------|
    // | rdi      |                         |
    // | rsi      | output pointer          |
    // | rdx      | paused stack pointer    |

    // the end of the function, always called into by resume()
    "1:", 
    // | register | value                  |
    // |----------|------------------------|
    // | rdi      |                        |
    // | rsi      | done pointer           |
    // | rdx      | input pointer          |
    // | rcx      | paused stack pointer   |
    inout("rdi") resume_stack => _,
    inout("rsi") output => done,
    out("rdx") input,
    out("rcx") paused_stack,
    out("rax") _,
    // clobber_abi("C")
    out("r8") _,    out("r9") _,    out("r10") _,   out("r11") _,
    out("r12") _,   out("r13") _,   out("r14") _,   out("r15") _,
    out("xmm0") _,  out("xmm1") _,  out("xmm2") _,  out("xmm3") _,
    out("xmm4") _,  out("xmm5") _,  out("xmm6") _,  out("xmm7") _,
    out("xmm8") _,  out("xmm9") _,  out("xmm10") _, out("xmm11") _,
    out("xmm12") _, out("xmm13") _, out("xmm14") _, out("xmm15") _,
  );
  (paused_stack, done, input)
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
pub unsafe fn resume(resume_stack: *mut usize, done: *mut u8, input: *const u8) -> (*mut usize, *const u8) {
  let paused_stack: *mut usize;
  let output: *const u8;
  asm!(
    // step 1: state preservation. we must spill our state to the stack so we may be resumed.
    "lea rax, [rip + 1f]", // calculate address of end of this function with forward ref
    "mov [rsp - 8],  rax", // save end of function as the return address
    "mov [rsp - 16], rbp", // save the frame pointer
    "mov [rsp - 24], rbx", // save llvm's nefarious porpoises register.
    "mov rcx, rsp", // save current stack pointer to rcx

    // step 2: state restoration (inverse of preservation) and returning
    "mov rsp, rdi",        // load the stack pointer to return to
    "mov rbx, [rdi - 24]",
    "mov rbp, [rdi - 16]",
    "mov rax, [rdi - 8]",
    "jmp rax",             // go there

    // | register | value                  |
    // |----------|------------------------|
    // | rdi      |                        |
    // | rsi      | done pointer           |
    // | rdx      | input pointer          |
    // | rcx      | paused stack pointer   |

    "1:", // the end of the function, always called into by suspend()
    // | register | value                  |
    // |----------|------------------------|
    // | rdi      |                        |
    // | rsi      | output pointer         |
    // | rdx      | paused stack pointer   |
    inout("rdi") resume_stack => _,
    inout("rsi") done => output,
    inout("rdx") input => paused_stack,
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
  (paused_stack, output)
}

/* Trampoline function
 * * is started in previous call frame
 * * calls the function above it on the stack
 * * adds a call frame for that function
 * * terminates the call chain for the debugger
 */

// On nightly, we can implement the tranpoline with a naked fn
#[cfg(feature="nightly")]
#[naked]
unsafe extern "C" fn trampoline() {
  asm!(
    // Stop unwinding at this frame. This directive tells the assembler that the old value of rip
    // (the instruction pointer) can no longer be restored.
    ".cfi_undefined rip",
    "jmp [rsp + 8]", // Call the return address
    options(noreturn)
  )
}

// In stable rust, we can use global_asm and an extern fn
#[cfg(not(feature="nightly"))]
extern "C" {
    fn trampoline();
}

#[cfg(not(feature="nightly"))]
core::arch::global_asm! {r"
.global trampoline
trampoline:
    jmp [rsp + 8]
"}
