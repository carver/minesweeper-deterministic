//! A virtual-time queue for the ripple animation. Items due at the same
//! time come out in the order they went in.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

pub type Millis = u64;

#[derive(Debug)]
pub struct Schedule<T> {
    now: Millis,
    next_seq: u64,
    queue: BinaryHeap<Reverse<Entry<T>>>,
}

#[derive(Debug)]
struct Entry<T> {
    due: Millis,
    seq: u64,
    item: T,
}

impl<T> PartialEq for Entry<T> {
    fn eq(&self, other: &Self) -> bool {
        (self.due, self.seq) == (other.due, other.seq)
    }
}
impl<T> Eq for Entry<T> {}
impl<T> PartialOrd for Entry<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for Entry<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.due, self.seq).cmp(&(other.due, other.seq))
    }
}

impl<T> Default for Schedule<T> {
    fn default() -> Self {
        Self {
            now: 0,
            next_seq: 0,
            queue: BinaryHeap::new(),
        }
    }
}

impl<T> Schedule<T> {
    pub fn now(&self) -> Millis {
        self.now
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn next_due(&self) -> Option<Millis> {
        self.queue.peek().map(|Reverse(e)| e.due)
    }

    pub fn after(&mut self, delay: Millis, item: T) {
        let entry = Entry {
            due: self.now + delay,
            seq: self.next_seq,
            item,
        };
        self.next_seq += 1;
        self.queue.push(Reverse(entry));
    }

    /// Pops the next item due by `until`, moving the clock to its due time.
    /// Once nothing more is due, moves the clock to `until`.
    pub fn pop_due(&mut self, until: Millis) -> Option<T> {
        match self.next_due() {
            Some(due) if due <= until => {
                let Reverse(entry) = self.queue.pop().expect("peeked");
                self.now = self.now.max(entry.due);
                Some(entry.item)
            }
            _ => {
                self.now = self.now.max(until);
                None
            }
        }
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pops_in_time_then_insertion_order() {
        let mut s = Schedule::default();
        s.after(20, "b");
        s.after(10, "a");
        s.after(20, "c");
        assert_eq!(s.pop_due(5), None);
        assert_eq!(s.now(), 5);
        assert_eq!(s.pop_due(100), Some("a"));
        assert_eq!(s.now(), 10);
        assert_eq!(s.pop_due(100), Some("b"));
        assert_eq!(s.pop_due(100), Some("c"));
        assert_eq!(s.pop_due(100), None);
        assert_eq!(s.now(), 100);
    }

    #[test]
    fn delays_are_relative_to_the_current_item() {
        let mut s = Schedule::default();
        s.after(10, 1);
        assert_eq!(s.pop_due(50), Some(1));
        s.after(10, 2);
        assert_eq!(s.pop_due(15), None);
        assert_eq!(s.pop_due(20), Some(2));
    }
}
