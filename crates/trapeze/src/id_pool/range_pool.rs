use std::cmp::Ordering;
use std::fmt::Debug;
use std::ops::{Bound, RangeBounds};

#[derive(Clone)]
struct Range {
    start: u32,
    end: u32,
}

pub struct RangePool {
    free: Vec<Range>,
}

impl Debug for RangePool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RangePool {{")?;
        for range in &self.free {
            write!(f, " [{}, {}]", range.start, range.end)?;
        }
        write!(f, " }}")?;
        Ok(())
    }
}

impl Default for RangePool {
    fn default() -> Self {
        Self::ranged(..)
    }
}

impl RangePool {
    pub fn ranged(range: impl RangeBounds<u32>) -> Self {
        let start = match range.start_bound() {
            Bound::Included(s) => *s,
            Bound::Excluded(s) => *s + 1,
            Bound::Unbounded => u32::MIN,
        };
        let end = match range.end_bound() {
            Bound::Included(e) => *e,
            Bound::Excluded(e) => *e - 1,
            Bound::Unbounded => u32::MAX,
        };
        let free = vec![Range { start, end }];
        Self { free }
    }

    pub fn new_id(&mut self) -> Option<u32> {
        let Some(range) = self.free.last_mut() else {
            // no more values available
            return None;
        };
        let id = range.start;
        if range.start < range.end {
            range.start += 1;
        } else {
            self.free.pop();
        }
        Some(id)
    }

    pub fn request_id(&mut self, id: u32) -> Option<u32> {
        let pos = self.free.binary_search_by(|range| {
            if range.start <= id && id <= range.end {
                Ordering::Equal
            } else {
                range.start.cmp(&id)
            }
        });
        let Ok(pos) = pos else {
            // `id` is not in the pool
            return None;
        };

        let range = self.free.get_mut(pos).unwrap();

        if range.start == range.end {
            // we consumed the last id in the range
            self.free.remove(pos);
        } else if range.start == id {
            // we just left shrink the existing range
            range.start += 1;
        } else if range.end == id {
            // we just right shrink the existing range
            range.end -= 1;
        } else {
            // we need to split the existing range in two
            let new = Range {
                start: range.start,
                end: id - 1,
            };
            range.start = id + 1;
            self.free.insert(pos, new);
        }

        Some(id)
    }

    pub fn return_id(&mut self, id: u32) -> Result<(), u32> {
        let pos = self.free.binary_search_by(|range| {
            if range.start <= id && id <= range.end {
                Ordering::Equal
            } else {
                range.start.cmp(&id)
            }
        });
        let Err(pos) = pos else {
            // `id` was already in the pool
            return Err(id);
        };

        // `id` was not found, but it would go in position `i`

        let adjacent_next = self.free.get(pos).is_some_and(|r| r.start == id + 1);

        let adjacent_prev = pos
            .checked_sub(1)
            .map(|i| &self.free[i])
            .is_some_and(|r| r.end == id - 1);

        match (adjacent_prev, adjacent_next) {
            (true, true) => {
                // adding the id merges the `prev` and `next` ranges
                self.free[pos - 1].end = self.free[pos].end;
                self.free.remove(pos);
            }
            (false, true) => {
                // adding the id extends the `next` range
                self.free[pos].start = id;
            }
            (true, false) => {
                // adding the id extends the `prev` range
                self.free[pos - 1].end = id;
            }
            (false, false) => {
                // the new id doesn't go in any adjacent range, add a new one
                self.free.insert(pos, Range { start: id, end: id });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn basic() {
        let mut pool = RangePool::ranged(..);

        assert_eq!(pool.request_id(1), Some(1));
        assert_eq!(pool.request_id(3), Some(3));
        assert_eq!(pool.request_id(5), Some(5));
        assert_eq!(pool.request_id(7), Some(7));
    }
}
