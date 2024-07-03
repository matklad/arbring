mod badcounter;

use std::collections::VecDeque;

struct Queue {
  buf: Box<[i32]>,
  inp: usize,
  outp: usize,
}

impl Queue {
  pub fn new(size: usize) -> Queue {
    Queue {
      buf: vec![0; size].into_boxed_slice(),
      inp: 0,
      outp: 0,
    }
  }

  fn push(&mut self, n: i32) {
    self.buf[self.inp] = n;
    self.inp = (self.inp + 1) % self.buf.len();
  }

  fn pop(&mut self) -> i32 {
    let ans = self.buf[self.outp];
    self.outp = (self.outp + 1) % self.buf.len();
    ans
  }

  fn len(&self) -> usize {
    (self.inp - self.outp) % self.buf.len()
  }
}

#[test]
fn test_queue() {
  arbtest::arbtest(|u| {
    let len_max = u.int_in_range(0..=10)?;
    let mut queue = Queue::new(len_max);
    let mut queue_model: VecDeque<i32> = VecDeque::new();

    while !u.is_empty() {
      match *u.choose(&["add", "remove"])? {
        "add" if queue_model.len() < len_max => {
          let x: i32 = u.arbitrary()?;
          queue_model.push_back(x);
          queue.push(x);
        }
        "remove" if queue_model.len() > 0 => {
          let x_model = queue_model.pop_front().unwrap();
          let x = queue.pop();
          assert_eq!(x_model, x);
        }
        "add" | "remove" => (),
        _ => unreachable!(),
      }
      assert_eq!(queue.len(), queue_model.len())
    }
    Ok(())
  });
}
