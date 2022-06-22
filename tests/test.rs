use stackle::{*, stack::*, switch::*};

fn adder(stack: *mut usize, arg: usize) {
  let mut ret = Switch { stack, arg };
  loop {
    ret = unsafe { switch(ret.stack, ret.arg + 1) };
  }
}

#[test]
fn adding() {
  unsafe {
    let s = AllocatorStack::new(8192);
    let c = link_closure_detached(s.end(), adder);
    let mut ret = Switch { stack: c, arg: 0 };
    for i in 0..1000 {
      ret = switch(ret.stack, ret.arg);
      assert_eq!(i + 1, ret.arg);
    }
  }
}

#[test]
fn adding_closure() {
  // this test is essentially a sanity check for the top frame of the
  // stack to make sure we haven't clobbered anything (which would
  // likely cause a segfault)
  unsafe {
    // this is just to give us something to move so it's a real closure
    let thing = (42usize, 42usize);
    let s = AllocatorStack::new(8192);
    let c = link_closure_detached(s.end(), |stack, arg| {
      // check our moved thing is still what we set it to.
      let thing = thing;
      assert_eq!(thing.0, 42);
      assert_eq!(thing.1, 42);
      let mut ret = Switch { stack, arg };
      loop {
        ret = switch(ret.stack, ret.arg + 1);
      }
    });
    let mut ret = Switch { stack: c, arg: 0 };
    for i in 0..1000 {
      ret = switch(ret.stack, ret.arg);
      assert_eq!(i + 1, ret.arg);
    }
  }
}
