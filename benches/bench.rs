use criterion::*;
use stackle::{*, stack::*, switch::*};

fn closure(mut stack: *mut usize, _value: usize) {
  loop {
    stack = unsafe { switch(stack, 0) }.stack
  }
}

fn linking_closure(c: &mut Criterion) {
  let mut group = c.benchmark_group("linking_closure");
  group.throughput(Throughput::Elements(1));
  group.bench_function(
    "stackle",
    |b| {
      unsafe {
        let s = AllocatorStack::new(8192);
        b.iter(|| {
          black_box(link_closure_detached(s.end(), closure));
        });
      }
    }
  );
}

fn ping_pong(c: &mut Criterion) {
  let mut group = c.benchmark_group("ping_pong");
  group.throughput(Throughput::Elements(1));
  group.bench_function(
    "stackle",
    |b| {
      unsafe {
        let s = AllocatorStack::new(8192);
        let c = link_closure_detached(s.end(), closure);
        let mut ret = Switch { stack: c, arg: 0 };
        b.iter(|| {
          ret = switch(ret.stack, ret.arg);
        })
      }
    }
  );
}

criterion_group!(
  benches,
  linking_closure,
  ping_pong,
);
criterion_main!(benches);

