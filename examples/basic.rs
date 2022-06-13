use stackle::{stack::*, switch::*};

fn st_adder(yielder: &Yield<()>) {
  let mut input = 0;
  loop {
    input = yielder.suspend(input + 1);
  }
}

fn main() {
  let s = unsafe { AllocatorStack::new(8192) };
  let c = unsafe { Coro::<()>::link(&s, st_adder) };
  let mut e = 0usize;
  for _ in 1..10 {
    e = unsafe { c.resume(e) }.unwrap();
  }
  println!("e: {}", e);
}
