use super::range_pool::RangePool;

pub struct OddRangePool {
    all: RangePool,
    odd: RangePool,
}

impl Default for OddRangePool {
    fn default() -> Self {
        OddRangePool {
            odd: RangePool::ranged(0..=(u32::MAX / 2)),
            all: RangePool::ranged(..),
        }
    }
}

impl OddRangePool {
    pub fn new_id(&mut self) -> Option<u32> {
        let odd_id = self.odd.new_id()?;
        let id = to_all(odd_id);
        self.all.request_id(id) // Invariant: this will always return Some(id)
    }

    pub fn request_id(&mut self, id: u32) -> Option<u32> {
        if let Some(odd_id) = to_odd(id) {
            self.odd.request_id(odd_id)?;
        }
        self.all.request_id(id)
    }

    pub fn return_id(&mut self, id: u32) -> Result<(), u32> {
        if let Some(odd_id) = to_odd(id) {
            self.odd.return_id(odd_id).map_err(|_| id)?;
        }
        self.all.return_id(id)
    }
}

fn to_odd(id: u32) -> Option<u32> {
    (id % 2 == 1).then(|| (id - 1) / 2)
}

fn to_all(id: u32) -> u32 {
    1 + 2 * id
}
