pub use std::io;
use super::Stack;
use std::fmt;
use std::ptr::null_mut;
use libc::{MAP_ANONYMOUS, MAP_FAILED, MAP_FIXED, MAP_PRIVATE, PROT_NONE, PROT_READ, PROT_WRITE, c_int, c_void};

#[derive(Debug)]
pub enum ParanoidError {
  GuardMapFailed(io::Error),
  StackMapFailed(*mut c_void, io::Error),
}

/// Puts a guard page before and after the stack to detect overflow and underflow.
pub struct ParanoidStack {
  start: *mut u8,
  size:  u32, // If you need gigabytes of stack you are doing it wrong
  page:  u32, // The page size. This will round us up to 2 words on 64-bit and 3 on 32-bit
}

impl fmt::Debug for ParanoidStack {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "ParanoidStack<{:x}-{:x}>", self.start as usize, self.end() as usize)
  }
}

unsafe impl Stack for ParanoidStack {
  fn end(&self) -> *mut usize {
    let size = self.size + self.page;
    unsafe { self.start.add(size as usize)}.cast()
  }
}

impl ParanoidStack {
  pub fn new(size: u32, page_size: PageSize) -> Result<Self, ParanoidError> {
    // No platform supports a double-guarded stack, or at least doesn't document doing so, so we
    // have to deal with at least one of the guard pages ourselves. Thus we start by allocating an
    // inaccessible region that covers both guard pages in addition to the stack.
    let size = page_size.round(size); // Rounding the page size helps with cross-platformness.
    let guard_size = page_size.0;
    let total_size = guard_size + guard_size + size;
    match unsafe { libc::mmap(null_mut(), total_size as usize, PROT_NONE, GUARD_FLAGS, -1, 0) } {
      MAP_FAILED => Err(ParanoidError::GuardMapFailed(io::Error::last_os_error())),
      not_ptr if not_ptr.is_null() => panic!("{}", MMAP_RETURNED_NULL),
      start => {
        // That went well. Now we have to allocate the useful portion of it.
        // * FreeBSD will allocate a guard page, but wants it included in the length and pointer.
        // * Frankly, I'm not sure for most of the others, they should improve their documentation
        //   where they add guard pages and should add guard page support otherwise. We will assume
        //   they want the start of the actual stack space and let users file bugs if it segfaults.
        #[cfg(target_os="freebsd")]
        let ptr = start; // FreeBSD wants the guard page included.
        #[cfg(not(target_os="freebsd"))]
        let ptr = unsafe { start.cast::<u8>().add(guard_size as usize) }.cast::<c_void>(); // Skip the guard page
        #[cfg(target_os="freebsd")]
        let stack_size = size + guard_size; // FreeBSD wants the guard page included.
        #[cfg(not(target_os="freebsd"))]
        let stack_size = size;
        // Now finally we actually do the mmap.
        match unsafe { libc::mmap(ptr, stack_size as usize, PROT, STACK_FLAGS, -1, 0) } {
          MAP_FAILED => Err(ParanoidError::StackMapFailed(start, io::Error::last_os_error())),
          not_ptr if not_ptr.is_null() => panic!("{}", MMAP_RETURNED_NULL),
          moved if moved != ptr => panic!("{}", MMAP_MOVED_FIXED),
          _ => Ok(ParanoidStack { start: start as *mut u8, size, page: page_size.0 })
        }
      }
    }
  }
}

impl Drop for ParanoidStack {
  fn drop(&mut self) {
    let size = self.size + self.page + self.page;
    unsafe { libc::munmap(self.start as *mut _, size as usize) };
  }
}

 #[derive(Debug)]
pub enum SafeError {
  GuardMapFailed(io::Error),
  StackMapFailed(*mut c_void, io::Error),
}

/// Puts a guard page before the stack to detect overflow.
pub struct SafeStack {
  start: *mut u8,
  size:  u32, // If you need gigabytes of stack you are doing it wrong
  page:  u32, // The page size. This will round us up to 2 words on 64-bit
}

impl fmt::Debug for SafeStack {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "SafeStack<{:x}-{:x}>", self.start as usize, self.end() as usize)
  }
}

unsafe impl Stack for SafeStack {
  fn end(&self) -> *mut usize {
    let size = self.size + self.page;
    unsafe { self.start.offset(size as isize)}.cast()
  }
}

impl SafeStack {
  pub fn new(size: u32, page_size: PageSize) -> Result<Self, SafeError> {
    let size = page_size.round(size); // Rounding the page size helps with cross-platformness.
    let guard_size = page_size.0;
    let total_size = size + guard_size;
    #[cfg(target_os="freebsd")] // FreeBSD is quite good actually.
    match unsafe { libc::mmap(null_mut(), total_size as usize, PROT_NONE, STACK_FLAGS, -1, 0) } {
      MAP_FAILED => Err(StackError::StackMapFailed(null_mut(), io::Error::last_os_error())),
      not_ptr if not_ptr.is_null() => panic!("{}", MMAP_RETURNED_NULL),
      start => Ok(SafeStack { start: start as *mut u8, size, page: page_size.0 }),
    }
    #[cfg(not(target_os="freebsd"))] // Oh no.
    match unsafe { libc::mmap(null_mut(), total_size as usize, PROT_NONE, GUARD_FLAGS, -1, 0) } {
      MAP_FAILED => Err(SafeError::GuardMapFailed(io::Error::last_os_error())),
      not_ptr if not_ptr.is_null() => panic!("{}", MMAP_RETURNED_NULL),
      start => {
        // That went well. Now we have to allocate the useful portion of it.
        let ptr = unsafe { start.cast::<u8>().add(guard_size as usize) }.cast::<libc::c_void>(); // Skip the guard page
        // Now finally we actually map the useful space
        match unsafe { libc::mmap(ptr, size as usize, PROT, STACK_FLAGS, -1, 0) } {
          MAP_FAILED => Err(SafeError::StackMapFailed(start, io::Error::last_os_error())),
          not_ptr if not_ptr.is_null() => panic!("{}", MMAP_RETURNED_NULL),
          moved if moved != ptr => panic!("{}: {}", MMAP_MOVED_FIXED, moved as usize),
          _ => Ok(SafeStack { start: start as *mut u8, size, page: page_size.0 }),
        }
      }
    }
  }
}

impl Drop for SafeStack {
  fn drop(&mut self) {
    let size = self.size + self.page;
    unsafe { libc::munmap(self.start as *mut _, size as usize) };
  }
}

#[repr(transparent)]
#[derive(Clone,Copy)]
/// A value holding the operating system's standard pagesize (probably 4k).
pub struct PageSize(u32);

impl PageSize {
  pub fn get() -> io::Result<PageSize> {
   match unsafe { libc::sysconf(libc::_SC_PAGESIZE) }{
      -1 => Err(io::Error::last_os_error()),
      size => Ok(PageSize(size as u32)),
    }
  }
  pub fn size(self) -> u32 { self.0 }
  pub fn round(self, size: u32) -> u32 {
    // Round up to the nearest page size
    let ps = self.0;
    let remainder = size & ps;
    let extra = ps & !!remainder; // !!: 0 = 0, n = usize::MAX
    size + extra
  }
}

const PROT: i32 = PROT_READ | PROT_WRITE;

const MMAP_RETURNED_NULL: &str =
  "Mmap returned null, which violates POSIX and certainly isn't sporting.";
const MMAP_MOVED_FIXED: &str =
  "Your OS doesn't even recognise MAP_FIXED, I can't really help you";

#[cfg(not(target_os="freebsd"))]
const GUARD_FLAGS: c_int = MAP_ANONYMOUS | MAP_PRIVATE;
#[cfg(target_os="freebsd")] // sounds like this is faster? not entirely sure.
const GUARD_FLAGS: c_int = MAP_ANONYMOUS | MAP_PRIVATE | libc::MAP_GUARD;

#[cfg(any(target_os="dragonflybsd", target_os="freebsd", target_os="linux", target_os="netbsd", target_os="openbsd"))]
const STACK_FLAGS: c_int = MAP_ANONYMOUS | MAP_PRIVATE | MAP_FIXED | libc::MAP_STACK;
#[cfg(not(any(target_os="dragonflybsd", target_os="freebsd", target_os="linux", target_os="netbsd", target_os="openbsd")))]
const STACK_FLAGS: c_int = MAP_ANONYMOUS | MAP_PRIVATE | MAP_FIXED;
