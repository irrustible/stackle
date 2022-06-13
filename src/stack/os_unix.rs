pub use std::io::Error;
use std::ptr::null_mut;

use libc::{MAP_ANONYMOUS, MAP_FAILED, MAP_SHARED, PROT_READ, PROT_WRITE};
const PROT: i32 = PROT_READ | PROT_WRITE;
const FLAGS: i32 = MAP_ANONYMOUS | MAP_SHARED;

const MMAP_RETURNED_NULL: &str =
  "Mmap returned null, which violates POSIX and certainly isn't sporting.";

pub struct OsStack {
  start: *mut u8,
  size:   u32, // If you need gigabytes of stack you are doing it wrong
}

unsafe impl super::Stack for OsStack {
  fn end(&self) -> *mut usize {
    unsafe { self.start.offset(self.size as isize)}.cast()
  }
}


impl OsStack {
  /// `mmap()`s a new `OsStack`, rounding the size to the nearest page size for the platform.
  pub fn new(size: u32, page_size: PageSize) -> Result<OsStack, Error> {
    let size = page_size.round(size);

    // All proper operating systems have some notion of mapping a stack, so our job is easy.
    #[cfg(any(target_os="dragonflybsd", target_os="freebsd", target_os="linux",
              target_os="netbsd",       target_os="openbsd"))] {
      match unsafe { libc::mmap(null_mut(), size as usize, PROT, FLAGS | libc::MAP_STACK, -1, 0) } {
        MAP_FAILED => Err(Error::last_os_error()),
        not_ptr if not_ptr.is_null() => panic!("{}", MMAP_RETURNED_NULL),
        ptr => Ok(OsStack { start: ptr as *mut _, size: size as u32}),
      }
    }
    #[cfg(not(any(target_os="dragonflybsd", target_os="freebsd", target_os="linux",
                  target_os="netbsd",       target_os="openbsd")))] {
      // While this platform, whatever it is (probably apple) claims to be unix, it clearly doesn't
      // really believe in it as it does not support MAP_STACK (or we haven't been able to determine
      // that it does). This is a massive pain because it means we need to construct the guard page
      // ourselves and we don't even know if MAP_FIXED will be respected.
      //
      // We will try anyway and hope for the best.
      const GUARD_FAIL:  &str = "Remapping the guard page failed. I can't even.";
      const GUARD_NIL:   &str = "Remapping the guard page returned nil. What is your OS smoking? I want some.";
      const GUARD_MOVED: &str = "Your operating system has a creative interpretation of POSIX and I'm giving up.";
    
      // First of all we must determine how much space to allocate. 
      let unguarded = size;
      let guard = page_size.size();
      let guarded = unguarded + guard;
      // We're going to mmap it twice. First for the full region including guard page.
      match libc::mmap(null_mut(), guarded as usize, PROT, FLAGS, -1, 0) {
        MAP_FAILED => Err(Error::last_os_error()),
        not_ptr if not_ptr == null_mut() => panic!("{}", MMAP_RETURNED_NULL),
        not_ptr => {
          // Now we're going to turn the guard page into a guard page.
          let ptr = not_ptr as *mut u8;
          match libc::mmap(ptr as *mut _, page_size.size() as usize, libc::PROT_NONE, FLAGS, -1, 0) {
            MAP_FAILED => panic!("{}", GUARD_FAIL),
            not_ptr if not_ptr.is_null() => panic!("{}", GUARD_NIL),
            not_ptr2 if not_ptr != not_ptr2 => panic!("{}", GUARD_MOVED),
            _ => Ok(OsStack {
              start: ptr.add(page_size.size()).cast(),
              size: unguarded,
            }),
          }
        }
      }    
    }
  }
}

impl Drop for OsStack {
  fn drop(&mut self) {
    #[cfg(any(target_os="dragonflybsd", target_os="freebsd", target_os="linux",
              target_os="netbsd",       target_os="openbsd"))] {
      unsafe { libc::munmap(self.start as *mut _, self.size as usize) };
    }
    #[cfg(not(any(target_os="dragonflybsd", target_os="freebsd", target_os="linux",
                  target_os="netbsd",       target_os="openbsd")))] {
      // To be tidy, we will undo the guard page we manually set up as well, even though this complicates matters...
      let guard_size = PageSize::get().unwrap().size(); // if it succeeded before, it will presumably succeed again...
      let ptr = self.start.sub(guard_size);
      let size = self.size + guard_size;
      unsafe { libc::munmap(ptr as *mut _, size); }
    }
  }
}

#[repr(transparent)]
#[derive(Clone,Copy)]
/// A value holding the operating system's standard pagesize (probably 4k).
pub struct PageSize(u32);

impl PageSize {
  pub fn get() -> Result<PageSize, Error> {
   match unsafe { libc::sysconf(libc::_SC_PAGESIZE) }{
      -1 => Err(Error::last_os_error()),
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

// Sorry, but if your alleged unix can't map a stack, you can't have nice things.
#[cfg(any(target_os="dragonflybsd", target_os="freebsd", target_os="linux", target_os="netbsd", target_os="openbsd"))]
pub struct OsStackConst<const SIZE: u32>(*mut u8);

#[cfg(any(target_os="dragonflybsd", target_os="freebsd", target_os="linux", target_os="netbsd", target_os="openbsd"))]
impl<const SIZE: u32> OsStackConst<SIZE> {
  pub fn new() -> Result<Self, Error> {
    match unsafe { libc::mmap(null_mut(), SIZE as usize, PROT, FLAGS | libc::MAP_STACK, -1, 0) } {
      MAP_FAILED => Err(Error::last_os_error()),
      not_ptr if not_ptr.is_null() => panic!("{}", MMAP_RETURNED_NULL),
      ptr => Ok(OsStackConst(ptr as *mut _)),
    }
  }
}
#[cfg(any(target_os="dragonflybsd", target_os="freebsd", target_os="linux", target_os="netbsd", target_os="openbsd"))]
unsafe impl<const SIZE: u32> super::Stack for OsStackConst<SIZE> {
  fn end(&self) -> *mut usize {
    unsafe { self.0.offset(SIZE as isize) }.cast()
  }
}

#[cfg(any(target_os="dragonflybsd", target_os="freebsd", target_os="linux", target_os="netbsd", target_os="openbsd"))]
impl<const SIZE: u32> Drop for OsStackConst<SIZE> {
  fn drop(&mut self) {
    unsafe { libc::munmap(self.0 as *mut _, SIZE as usize) };
  }
}
