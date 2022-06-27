# Stackle

Low level utilities for implementing green threads and coroutines.

## Status: alpha

Things may still change and we only support one platform so far.

I believe I've dealt with all the segfaults now, but it's still too
early to be sure.

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

| OS           | Compatibility | Tested on |
|--------------|---------------|-----------|
| DragonflyBSD | yes           | NO        |
| FreeBSD      | 12+           | NO        |
| Linux        | yes           | x86-64    |
| NetBSD       | yes           | NO        |
| OpenBSD      | yes           | NO        |

Note: "yes" here means that the API doesn't appear to have changed significantly for our purposes in
many years so within reason, any version ought to work.

More to come:
* x86-64 windows
* x86 unix + windows
* aarch64 unix + windows
* arm unix (+ windows?)
* riscv unix

## Limitations

* Detached stacks start at a trampoline function. Avoiding this would mean requiring the user to
  write each function they wanted to spawn on the new stack in assembly.

## Performance

### A note on inlining

In order to achieve great hot loop performance, we make heavy use of `#[inline(always)]`, enabling
the cpu to branch predict through them quite well and reducing runtime by up to 3x in the case of a
double misprediction (switch to and switch from).

If you are freely switching dynamically between lots of coroutines, you are unlikely to be able to
benefit from this, thus you might consider wrapping these functions without an inline annotation to
reduce total code size and compilation time if these are of concern.

### Benchmarks

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

To put this into context, atomically incrementing an arc takes longer than a pair of context
switches on my machine. So at least on a modern machine with good branch prediction, seems about optimal

## Snark

At a minimum, a stack is just an appropriately aligned chunk of memory. Depending on your platform,
it may also mean some sort of flag being set in the kernel for a particular space, too.

Another desirable feature of a stack is guard pages, pages that are guaranteed to trigger a segfault
when accessed. While a segfault might seem like a bad thing, you should never exceed the maximum
stack size anyway, and the alternative might be overwriting memory that you shouldn't.

Having spent far too long fighting different operating systems, i'm inclined to rank them in terms
of how good I think their mmap support is for our purposes:

* FreeBSD: Good, albeit slightly confusing.
  * `MAP_STACK` means something useful on FreeBSD. It uses page-at-a-time faulting, terminates the
    stack with a guard and defines whether the guard is included in the length.
  * No guard page after the stack in memory.
  * Explicit `MAP_GUARD` flag which sounds like it might be cheaper?. Not sure from the docs.
* OpenBSD: Not sure.
  * `MAP_STACK` exists but has no useful information in the documentation.
  * Recommends against the use of `MAP_FIXED` (needed to implement guard pages yourself).
* DragonflyBSD: Probably okay? Docs are confusing.
  * `MAP_STACK` seems to describe a page-at-a-time faulting mechanism, but the wording isn't
    entirely clear. It's not even clear about whether the guard page size should be included in the
    requested length (I think not).
* NetBSD: Bad.
  * `MAP_STACK` doesn't mean anything, it's just a future-proofing in case it does one day.
  * Recommends against the use of `MAP_FIXED` (needed to implement guard pages yourself).
* Linux: Really bad.
  * `MAP_STACK` doesn't mean anything, it's just a future-proofing in case it does one day.
  * `MAP_GROWSDOWN` describes a page-at-a-time faulting mechanism, but multiple sources say it has
    never worked properly, isn't really used and shouldn't be used.
* Apple: Hopeless, but honest.
  * Doesn't even have `MAP_STACK`. Or much of anything, really.

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
