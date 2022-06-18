use stackle::{*, stack::*, switch::*};

fn adder(paused: *mut usize, value: usize) {
  let mut ret = (paused, value);
  loop {
    ret = unsafe { suspend(ret.0, ret.1 + 1) };
  }
}

#[test]
fn adding() {
  unsafe {
    let s = AllocatorStack::new(8192);
    let c = link_closure(adder, s.end());
    let mut ret = (c, 0usize);
    for i in 0..1000 {
      ret = resume(ret.0, ret.1);
      assert_eq!(i + 1, ret.1);
    }
  }
}
