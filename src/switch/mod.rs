mod arch;

use crate::stack::Stack;
use std::{any::Any, cell::Cell, marker::PhantomData, ptr::null};
use ointers::Ointer;

type InitFn =  unsafe extern "C" fn(*const u8, *mut usize);

pub type Panic = Box<dyn Any + Send>;

trait Simple {
  fn into_usize(self) -> usize;
  fn from_usize(u: usize) -> Self;
}

/// Unlike most coroutines, this one is only typed by its return value.
pub struct Coro<R> {
  stack:  Cell<SP>,
  _phant: PhantomData<R>,
}

/// Wraps an item for transfer on the stack, erasing it to a *const u8;
#[macro_export]
macro_rules! give {
  ($name:ident : $type:ty) => {
    let $name = core::mem::ManuallyDrop::new($name);
    let $name = (&$name as *const core::mem::ManuallyDrop<$type>).cast::<u8>();
  }
}

/// Moves an item that was give!()n before stack switch, via pointer read.
#[macro_export]
macro_rules! take {
  ($name:ident : $type:ty) => { $name.cast::<$type>().read() }
}

/// Should be equivalent to reserving space on the stack
#[macro_export]
macro_rules! reserve {
  ($name:ident : $type:ty) => {
    let $name: core::cell::UnsafeCell<core::mem::MaybeUninit<$type>>
      = core::cell::UnsafeCell::new(core::mem::MaybeUninit::uninit());
  }
}

/// Retrieves an erased pointer to a reserved thing.
#[macro_export]
macro_rules! reserved {
  ($name:ident) => { $name.get().cast::<u8>() }
}

/// Retrieves the reserved item, which is expected to be filled, via pointer read.
#[macro_export]
macro_rules! promised {
  ($name:ident : $type:ty) => { $name.get().read().assume_init() }
}

impl<R> Coro<R> {

  /// Creates a new coroutine on the provided stack, executing the
  /// provided closure with the provided argument.
  ///
  /// # Safety
  ///
  /// * The stack must be fresh.
  /// * The stack must not be used by another thread during the duration of this call.
  #[inline(always)]
  pub unsafe fn link<F, S>(stack: &S, fun: F) -> Coro<R>
  where S: Stack, F: FnOnce(&Yield<R>) -> R {
    give!(fun: F);
    let stack = arch::link(fun, Self::entrypoint::<F, S>, stack.end());
    Coro {
      stack: Cell::new(SP::new(stack)),
      _phant: PhantomData,
    }
  }

  /// This is the initial entrypoint function
  unsafe extern "C" fn entrypoint<F, S>(fun: *const u8, stack_ptr: *mut usize)
  where S: Stack, F: FnOnce(&Yield<R>) -> R {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    // step 1: move the closure to the new stack and switch back.
    let fun = take!(fun: F);
    // step 2: call the closure, trapping any panics and return indirectly.
    let (new_stack, done, _input) = arch::suspend(stack_ptr, null());
    let yielder = Yield::<R> {
      stack: Cell::new(new_stack),
      done:  Cell::new(done as *mut _),
    };
    let ret = catch_unwind(AssertUnwindSafe(|| fun(&yielder)));
    yielder.done.get().write(ret);
    arch::suspend(yielder.stack.get(), null());
  }

  /// # Safety
  ///
  /// * The stack must not be used by another thread during the duration of this call.
  #[inline(always)]
  pub unsafe fn resume<I, O>(&self, input: I) -> Result<O, Result<R, Panic>> {
    reserve!(result: Result<R, Panic>);
    give!(input: I);
    let sp = self.stack.get();
    let stack_ptr = sp.as_ptr();
    // self.stack.set(sp.start()); // Mark ourselves started
    let (stack_ptr, output) = arch::resume(stack_ptr, reserved!(result), input);
    if !output.is_null() {
      self.stack.set(sp.replace(stack_ptr));
      Ok(take!(output: O))
    } else {
      self.stack.set(sp.finish());
      Err(promised!(result: Result<R, Panic>))
    }
  }

  /// # Safety
  ///
  /// * The stack must not be used by another thread during the duration of this call.
  #[inline(always)]
  pub unsafe fn resume_direct(&self, done: *mut u8, input: *const u8) -> *const u8 {
    let sp = self.stack.get();
    let stack_ptr = sp.as_ptr();
    // self.stack.set(sp.start()); // Mark ourselves started
    let (stack_ptr, output) = arch::resume(stack_ptr, done, input);
    self.stack.set(sp.replace(stack_ptr));
    output
  }
}

pub struct Yield<R> {
  stack: Cell<*mut usize>,
  done:  Cell<*mut Result<R, Panic>>,
}

impl<R> Yield<R> {
  #[inline(always)]
  pub fn suspend<I, O>(&self, output: O) -> I {
    give!(output: O);
    let (stack, done, input) = unsafe { arch::suspend(self.stack.get(), output) };
    self.stack.set(stack);
    self.done.set(done as *mut _);
    unsafe { take!(input: I) }
  }
  #[inline(always)]
  pub unsafe fn suspend_direct(&self, output: *const u8) -> *const u8 {
    let (stack, done, input) = arch::suspend(self.stack.get(), output);
    self.stack.set(stack);
    self.done.set(done as *mut _);
    input
  }
}

#[derive(Clone,Copy)]
#[repr(transparent)]
struct SP(Ointer<usize, 2, false, 0>);

const STARTED: usize = isize::MIN as usize;
const FINISHED: usize = isize::MIN as usize >> 1;

impl SP {
  #[inline(always)]
  unsafe fn new(ptr: *mut usize) -> SP { SP(Ointer::new(ptr).steal(0)) }

  #[inline(always)]
  fn as_ptr(&self) -> *mut usize { self.0.as_ptr() }

  #[inline(always)]
  fn start(self) -> Self { SP(self.0.steal(STARTED | self.0.stolen())) }

  // #[inline(always)]
  // fn started(&self) -> bool { 0 != self.0.stolen() & STARTED }

  #[inline(always)]
  fn finish(self) -> Self { SP(self.0.steal(FINISHED | self.0.stolen())) }

  // #[inline(always)]
  // fn finished(&self) -> bool { 0 != self.0.stolen() & FINISHED }

  #[inline(always)]
  fn replace(self, with: *mut usize) -> Self {
    SP(unsafe { Ointer::new(with).steal(self.0.stolen()) })
  }
}
  
