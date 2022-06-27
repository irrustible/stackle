use criterion::*;
use stackle::{stack::*, switch::*};

fn closure(mut stack: *mut usize, _value: usize) {
  loop {
    stack = unsafe { switch(stack, 0) }.stack
  }
}

// this takes about as long as linking on my machine!
fn get_page_size(c: &mut Criterion) {
  let mut group = c.benchmark_group("get_page_size");
  group.throughput(Throughput::Elements(1));
  group.bench_function(
    "stackle",
    |b| { b.iter(|| black_box(PageSize::get())) }
  );
}

fn alloc_stack(c: &mut Criterion) {
  let mut group = c.benchmark_group("alloc_stack");
  group.throughput(Throughput::Elements(1));
  group.bench_function(
    "allocator",
    |b| b.iter(|| black_box(unsafe { AllocatorStack::new(8192) }))
  );
  group.bench_function(
    "safe",
    |b| {
      let p = PageSize::get().unwrap();
      b.iter(|| black_box(SafeStack::new(8192, p)))
    }
  );
  group.bench_function(
    "paranoid",
    |b| {
      let p = PageSize::get().unwrap();
      b.iter(|| black_box(SafeStack::new(8192, p)))
    }
  );
}

fn linking_closure_detached(c: &mut Criterion) {
  let mut group = c.benchmark_group("linking_closure");
  group.throughput(Throughput::Elements(1));
  group.bench_function(
    "allocator",
    |b| {
      unsafe {
        let s = AllocatorStack::new(8192);
        b.iter(|| {
          black_box(link_closure_detached(s.end(), closure));
        });
      }
    }
  );
  group.bench_function(
    "safe",
    |b| {
      let p = PageSize::get().unwrap();
      let s = SafeStack::new(8192, p).unwrap();
      b.iter(|| {
        black_box(unsafe { link_closure_detached(s.end(), closure) });
      });
    }
  );
  group.bench_function(
    "paranoid",
    |b| {
      let p = PageSize::get().unwrap();
      let s = ParanoidStack::new(8192, p).unwrap();
      b.iter(|| {
        black_box(unsafe { link_closure_detached(s.end(), closure) });
      });
    }
  );
}

fn switching(c: &mut Criterion) {
  let mut group = c.benchmark_group("ping_pong");
  group.throughput(Throughput::Elements(1));
  group.bench_function(
    "allocator",
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
  group.bench_function(
    "safe",
    |b| {
      let p = PageSize::get().unwrap();
      let s = SafeStack::new(8192, p).unwrap();
      let c = unsafe { link_closure_detached(s.end(), closure) };
      let mut ret = Switch { stack: c, arg: 0 };
      b.iter(|| {
        ret = unsafe { switch(ret.stack, ret.arg) };
      })
    }
  );
  group.bench_function(
    "paranoid",
    |b| {
      let p = PageSize::get().unwrap();
      let s = ParanoidStack::new(8192, p).unwrap();
      let c = unsafe { link_closure_detached(s.end(), closure) };
      let mut ret = Switch { stack: c, arg: 0 };
      b.iter(|| {
        ret = unsafe { switch(ret.stack, ret.arg) };
      })
    }
  );
}

criterion_group!(
  benches,
  get_page_size,
  alloc_stack,
  linking_closure_detached,
  switching,
);
criterion_main!(benches);

