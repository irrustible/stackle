#[cfg(feature="std")]
use std::alloc::{alloc, dealloc, Layout};
#[cfg(not(feature="std"))]
use alloc::alloc::{alloc, dealloc, Layout};

/// A dynamically-sized GlobalAlloc-allocated stack
pub struct AllocatorStack {
  start: *mut u8,
  size:   u32, // If you need gigabytes of stack you are doing it wrong
}

#[cfg(any(
  // https://community.arm.com/arm-community-blogs/b/architectures-and-processors-blog/posts/using-the-stack-in-aarch32-and-aarch64
  target_arch="aarch64",
  // https://agner.org/optimize/calling_conventions.pdf
  target_arch="x86_64",
  // https://github.com/riscv-collab/riscv-gcc/issues/61
  target_arch="riscv",   target_arch="riscv64",
  // https://en.wikipedia.org/wiki/X86_calling_conventions#cdecl
  all(target_arch="x86", unix),                 
))]
const ALIGN: usize = 16;

// https://community.arm.com/arm-community-blogs/b/architectures-and-processors-blog/posts/using-the-stack-in-aarch32-and-aarch64
#[cfg(any(target_arch="arm"))]
const ALIGN: usize = 8;

// https://agner.org/optimize/calling_conventions.pdf
#[cfg(all(target_arch="x86", windows))] 
const ALIGN: usize = 4;

impl AllocatorStack {
  /// Allocates a new stack on the heap with the given size.
  ///
  /// # Safety
  ///
  /// It's actually the drop that's unsafe:
  /// * You promise not to drop it while it's being used.
  /// * Ideally, if it has been used, unwind it first.
  pub unsafe fn new(size: u32) -> AllocatorStack {
    let layout =  Layout::from_size_align_unchecked(size as usize, ALIGN);
    let start = alloc(layout);
    AllocatorStack { start, size }
  }
}

impl Drop for AllocatorStack {
  fn drop(&mut self) {
    let layout = unsafe { Layout::from_size_align_unchecked(self.size as usize, ALIGN) };
    unsafe { dealloc(self.start, layout) }
  }
}

unsafe impl super::Stack for AllocatorStack {
  fn end(&self) -> *mut usize {
    unsafe { self.start.offset(self.size as isize)}.cast()
  }
}

/// A const-sized GlobalAlloc-allocated stack
pub struct AllocatorStackConst<const SIZE: u32>(*mut u8);

impl<const SIZE: u32> AllocatorStackConst<SIZE> {
  /// Allocates a new stack on the heap with the given size.
  ///
  /// # Safety
  ///
  /// It's actually the drop that's unsafe:
  /// * You promise not to drop it while it's being used.
  /// * Ideally, if it has been used, unwind it first.
  pub unsafe fn new() -> Self {
    let layout =  Layout::from_size_align_unchecked(SIZE as usize, ALIGN);
    let start = alloc(layout);
    AllocatorStackConst(start)
  }
}

impl<const SIZE: u32> Drop for AllocatorStackConst<SIZE> {
  fn drop(&mut self) {
    unsafe { libc::munmap(self.0 as *mut _, SIZE as usize) };
  }
}

unsafe impl<const SIZE: u32> super::Stack for AllocatorStackConst<SIZE> {
  fn end(&self) -> *mut usize {
    unsafe { self.0.offset(SIZE as isize)}.cast()
  }
}
