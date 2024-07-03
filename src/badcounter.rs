use std::{
  cell::RefCell,
  sync::{
    atomic::Ordering::{self, SeqCst},
    mpsc, Arc, Condvar, Mutex,
  },
  thread::{self, ScopedJoinHandle},
};

use arbtest::arbitrary::size_hint::or;

#[derive(Default)]
struct BadCounter {
  value: AtomicU32,
}

impl BadCounter {
  fn increment(&self) {
    // self.value.fetch_add(1, SeqCst);
    let value = self.value.load(SeqCst);
    self.value.store(value + 1, SeqCst);
  }

  fn get(&self) -> u32 {
    self.value.load(SeqCst)
  }
}

#[test]
fn test_bad_counter() {
  arbtest::arbtest(|rng| {
    let counter = BadCounter::default();
    let mut counter_model: u32 = 0;

    thread::scope(|scope| {
      let mut t1 = controlled_thread(scope, &counter);
      let mut t2 = controlled_thread(scope, &counter);

      while !rng.is_empty() {
        for t in [&mut t1, &mut t2] {
          if rng.arbitrary()? {
            if t.is_blocked() {
              t.unblock()
            } else {
              t.act(|it| it.increment());
              counter_model += 1;
            }
          }
        }
      }
      drop((t1, t2));

      assert_eq!(counter.get(), counter_model);
      Ok(())
    })
  });
}

#[derive(PartialEq, Eq)]
enum Status {
  Ready,
  Running,
  Blocked,
}

struct Shared {
  status: Mutex<Status>,
  cv: Condvar,
}

thread_local! {
    static TLS: RefCell<Option<Arc<Shared>>> = RefCell::new(None);
}

impl Shared {
  fn get() -> Option<Arc<Shared>> {
    TLS.with(|it| it.borrow().clone())
  }

  fn block(&self) {
    let mut guard = self.status.lock().unwrap();
    assert!(*guard == Status::Running);
    *guard = Status::Blocked;
    self.cv.notify_all();
    let guard = self
      .cv
      .wait_while(guard, |it| *it == Status::Blocked)
      .unwrap();
    assert!(*guard == Status::Running);
  }
}

#[derive(Default)]
struct AtomicU32 {
  inner: std::sync::atomic::AtomicU32,
}

impl AtomicU32 {
  fn load(&self, ordering: Ordering) -> u32 {
    if let Some(shared) = Shared::get() {
      shared.block()
    }
    let result = self.inner.load(ordering);
    if let Some(shared) = Shared::get() {
      shared.block()
    }
    result
  }
  fn store(&self, value: u32, ordering: Ordering) {
    if let Some(shared) = Shared::get() {
      shared.block()
    }
    self.inner.store(value, ordering);
    if let Some(shared) = Shared::get() {
      shared.block()
    }
  }
  fn fetch_add(&self, value: u32, ordering: Ordering) {
    if let Some(shared) = Shared::get() {
      shared.block()
    }
    self.inner.fetch_add(value, ordering);
    if let Some(shared) = Shared::get() {
      shared.block()
    }
  }
}

fn controlled_thread<'scope, State: 'scope + Send>(
  scope: &'scope thread::Scope<'scope, '_>,
  mut state: State,
) -> ControlledThread<'scope, State> {
  let shared = Arc::new(Shared {
    status: Mutex::new(Status::Ready),
    cv: Condvar::new(),
  });
  let (sender, receiver) = mpsc::channel::<Box<dyn FnOnce(&mut State) + Send>>();
  let handle = scope.spawn({
    let shared = Arc::clone(&shared);
    move || {
      TLS.with(|it| *it.borrow_mut() = Some(shared.clone()));
      for act in receiver {
        let guard = shared.status.lock().unwrap();
        assert!(*guard == Status::Running);
        drop(guard);
        act(&mut state);
        let mut guard = shared.status.lock().unwrap();
        assert!(*guard == Status::Running);
        *guard = Status::Ready;
        shared.cv.notify_all()
      }
    }
  });

  ControlledThread {
    shared,
    sender: Some(sender),
    is_blocked: false,
    handle: Some(handle),
  }
}

struct ControlledThread<'scope, State: 'scope> {
  handle: Option<thread::ScopedJoinHandle<'scope, ()>>,
  shared: Arc<Shared>,
  sender: Option<mpsc::Sender<Box<dyn FnOnce(&mut State) + Send>>>,
  is_blocked: bool,
}

impl<'scope, State: 'scope> ControlledThread<'scope, State> {
  fn act(&mut self, f: impl FnOnce(&mut State) + Send + 'static) {
    let mut guard = self.shared.status.lock().unwrap();
    assert!(*guard == Status::Ready);
    *guard = Status::Running;
    drop(guard);
    self.sender.as_ref().unwrap().send(Box::new(f)).unwrap();
    let guard = self.shared.status.lock().unwrap();
    let guard = self
      .shared
      .cv
      .wait_while(guard, |it| *it == Status::Running)
      .unwrap();
    self.is_blocked = *guard == Status::Blocked;
  }

  fn is_blocked(&self) -> bool {
    self.is_blocked
  }

  fn unblock(&mut self) {
    let mut guard = self.shared.status.lock().unwrap();
    assert!(*guard == Status::Blocked);
    *guard = Status::Running;
    self.shared.cv.notify_all();
    let guard = self
      .shared
      .cv
      .wait_while(guard, |it| *it == Status::Running)
      .unwrap();
    self.is_blocked = *guard == Status::Blocked;
  }
}

impl<'scope, State: 'scope> Drop for ControlledThread<'scope, State> {
  fn drop(&mut self) {
    while self.is_blocked {
      self.unblock();
    }
    self.sender.take().unwrap();
    self.handle.take().unwrap().join();
  }
}
