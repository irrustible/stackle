use criterion::*;
use stackle::{*, stack::*, switch::*};

fn closure(mut paused: *mut usize, _value: usize) {
  loop {
    paused = unsafe { suspend(paused, 0) }.0;
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
          black_box(link_closure(closure, s.end()));
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
        let c = link_closure(closure, s.end());
        let mut ret = (c, 0usize);
        b.iter(|| {
          ret = resume(ret.0, ret.1);
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

