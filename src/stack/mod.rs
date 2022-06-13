
/// # Safety
///
/// * end() must return an appropriately aligned pointer.
pub unsafe trait Stack {
  /// Returns a pointer past the end of the stack.
  fn end(&self) -> *mut usize;
}

#[cfg(any(feature="alloc", feature="std"))]
mod allocator;
#[cfg(any(feature="alloc", feature="std"))]
pub use allocator::*;

#[cfg(all(unix,feature="std"))]
mod os_unix;
#[cfg(all(unix,feature="std"))]
pub use os_unix::*;

// #[cfg(all(windows,feature="std"))]
// mod os_windows;
// #[cfg(all(windows,feature="std"))]
// pub use os_windows::*;
