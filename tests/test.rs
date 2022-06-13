use stackle::{*, stack::*, switch::*};

fn direct_adder(yielder: &Yield<()>) {
  let mut input = 0usize;
  loop {
    input = unsafe { yielder.suspend_direct((input + 1) as *const _) as usize };
  }
}

fn indirect_adder(yielder: &Yield<()>) {
  let mut input = 0usize;
  loop {
    input = yielder.suspend(input + 1);
  }
}

#[test]
fn direct() {
  let s = unsafe { AllocatorStack::new(8192) };
  let c = unsafe { Coro::<()>::link(&s, direct_adder) };
  let mut e = 0usize;
  reserve!(result: Result<(), Panic>);
  e = unsafe { c.resume_direct(reserved!(result), e as *const _) } as usize;
  assert_eq!(e, 1);
  e = unsafe { c.resume_direct(reserved!(result), e as *const _) } as usize;
  assert_eq!(e, 2);
  e = unsafe { c.resume_direct(reserved!(result), e as *const _) } as usize;
  assert_eq!(e, 3);
  e = unsafe { c.resume_direct(reserved!(result), e as *const _) } as usize;
  assert_eq!(e, 4);
  e = unsafe { c.resume_direct(reserved!(result), e as *const _) } as usize;
  assert_eq!(e, 5);

  // for ret in 1..10 {
  // }
}

#[test]
fn simple() {
  let s = unsafe { AllocatorStack::new(8192) };
  let c = unsafe { Coro::<()>::link(&s, indirect_adder) };
  let mut e = 0usize;
  e = unsafe { c.resume(e) }.unwrap();
  assert_eq!(e, 1);
  e = unsafe { c.resume(e) }.unwrap();
  assert_eq!(e, 2);
  e = unsafe { c.resume(e) }.unwrap();
  assert_eq!(e, 3);
  // for ret in 1..10 {
  // }
}
