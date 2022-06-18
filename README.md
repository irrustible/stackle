# Stackle

Low level utilities for implementing green threads and coroutines.

## Status: alpha

Things may still change and we only support one platform so far.

I have suspicions that the entry frames are set up wrong

## Usage

```rust
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
    let s = OsStack::new(128 * 1024); // 128k seems reasonable.
    let c = link_closure(adder, s.end());
    let mut ret = (c, 0usize);
    for i in 0..1000 {
      ret = resume(ret.0, ret.1);
      assert_eq!(i + 1, ret.1);
    }
  }
}
```

## Platform support

* x86-64 unix

More to come:
* x86-64 windows
* x86 unix + windows
* aarch64 unix + windows
* arm unix + windows
* riscv unix

## Performance

A short benchmark suite is included for the stack linking and switching primitives.

Each benchmark involves resuming and suspending a closure, thus we're
actually measuring *two* context switches per iteration.

Here's a recent run on my primary dev machine (an AMD Ryzen 9 3900X):

```
linking_closure/stackle time:   [3.4150 ns 3.4197 ns 3.4257 ns]                                     
                        thrpt:  [291.91 Melem/s 292.42 Melem/s 292.83 Melem/s]
ping_pong/stackle       time:   [2.0146 ns 2.0158 ns 2.0171 ns]                               
                        thrpt:  [495.76 Melem/s 496.09 Melem/s 496.37 Melem/s]
```

To put this into context, atomically incrementing an arc takes longer
than a pair of context switches on my machine. So at least on a modern
machine with good branch prediction, I doubt you could do better than
this.

Caveats:

These figures represent hot loop performance. We are always resuming the same coroutine and my CPU
appears to predict fairly well through the context switches. Your results may vary.

When resuming lots of coroutines, you won't get these figures. On my machine, branch prediction is the dominant
determinant of execution time, with a double misprediction up to tripling the time of an iteration.

## Copyright and License

Copyright (c) 2022 James Laver

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
