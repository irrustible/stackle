//! x86 is a bit limited on registers, so we have to be slightly creative. We use the fastcall ABI
//! to get two parameters into registers and the third goes on the stack.
use crate::switch::{InitFn, Switch};
use core::arch::asm;

/// Links a coroutine with a new stack, starting a new call stack at a trampoline.
///
/// # Safety
///
/// * `stack` must be a properly aligned pointer to the end of a region.
/// * `stack` must either have a guard page allocated or not overflow.
#[inline(always)]
pub unsafe extern "fastcall" fn link_detached(
  fun: InitFn,           // the function that the trampoline will call
  arg: usize,            // probably a pointer to a closure
  mut stack: *mut usize, // the end of a stack region.
) -> *mut usize {
  // Step 1: setting up the new stack. We do this in rust space to reduce register pressure.
  unsafe {
    stack.sub(1).write(trampoline as usize);
    stack.sub(2).write(fun as usize);
  }
  // the new stack should now look like this:
  // | end rel | data                |
  // |---------|---------------------|
  // | -4      | trampoline function |
  // | -8      | entrypoint function |
  asm!(

    // step 2: state preservation. we must spill our state to the stack so we may be resumed.
    "lea esp, [esp - 12]", // make space for the 3 words on the stack
    "mov [esp - 4],  ebp", // save the frame pointer in case it is used
    "mov [esp - 8],  ebx", // save llvm's nefarious porpoises register.
    "lea ecx, [eip + 2f]", // calculate address of end of this function with forward ref
    "mov [esp - 12], ecx", // save end of function as the return address
    // our stack should now look like this:
    // | esp rel | data           |
    // |---------|----------------|
    // | -4      | frame pointer  |
    // | -8      | llvm obscurity |
    // | -12     | return address |

    // step 3: setting up parameters
    "mov ecx, esp", // current stack pointer -> arg 1

    // argument layout should now be:
    // | register | value                   |
    // |----------|-------------------------|
    // | rdi      | fun (closure parameter) |
    // | rsi      | rsp (our stack pointer) |
    
    // step 4: calling trampoline on the new stack.
    "xor ebx, ebx",        // zero out rbx
    "xor ebp, ebp",        // zero out rbp (meaning "top of call chain")
    "lea esp, [eax - 16]", // set the correct stack pointer
    "jmp [eax - 8]",       // switch to trampoline
    
    // End of function, as taken in first instruction. register layout should now be:
    // our internal calling convention is this:
    // | register | value                   |
    // |----------|-------------------------|
    // | edx      | arg                     |
    // | eax      | paused stack pointer    |

    "2:",
    inout("edx") arg => _,
    inout("eax") stack,
    clobber_abi("fastcall")
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
pub unsafe extern "fastcall" fn switch(mut stack: *mut usize, mut arg: usize) -> Switch {
  asm!(
    // spill to stack
    "lea esp, [esp - 12]", // make space for the 3 words on the stack
    "mov [esp - 4],  ebp", // save the frame pointer (we aren't allowed to clobber it)
    "mov [esp - 8],  ebx", // save llvm's nefarious porpoises register (no clobbering)
    "lea eax, [eip + 2f]", // calculate address of end of this function with forward ref
    "mov [esp - 12], eax", // save end of function as the return address

    // switch stacks
    "mov eax, esp",        // save current stack pointer into eax
    "mov esp, ecx",        // load new stack pointer

    // step 2: state restoration (inverse of preservation) and branching
    "mov eax, [ecx - 12]",
    "mov ebp, [ecx - 8]",
    "mov eax, [ecx - 4]",
    "lea esp, [esp + 12]", // reset the stack pointer
    "jmp eax",

    // our internal calling convention is this:
    // | register | value                   |
    // |----------|-------------------------|
    // | edx      | arg                     |
    // | eax      | paused stack pointer    |

    // the end of the function, always called into by resume()
    "2:", 
    inout("ecx") stack => _,
    inout("edx") arg => _,
    out("eax") stack,
    clobber_abi("fastcall")
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
  ".cfi_undefined eip",    // stop unwinding at this frame
  ".cfi_undefined esp",    // stop the call chain at this frame (for gdb)
  "call [esp]",            // call the function in a new stack frame.
  ".cfi_endproc"           // function epilogue
);
