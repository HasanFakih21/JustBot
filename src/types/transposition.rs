use std::sync::atomic::{AtomicPtr, AtomicU8, AtomicUsize, Ordering};

use crate::types::{Score, moves::Move};

const TT_DEFAULT_SIZE: usize = 16;
const MEGABYTE: usize = 1024 * 1024;
const MAX_AGE: u8 = 31;

const SIZE_OF_CLUSTER: usize = std::mem::size_of::<Cluster>();
const NUM_ENTRIES_PER_CLUSTER: usize = 3;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bound {
    None,
    Exact,
    Upper,
    Lower,
}

#[derive(Debug, Clone, Copy)]
pub struct Flags(u8);

impl Flags {
    pub fn new(pv: bool, bound: Bound, age: u8) -> Self {
        debug_assert!(age <= MAX_AGE);

        Flags(pv as u8 | (bound as u8) << 1 | age << 3)
    }

    pub const fn bound(&self) -> Bound {
        match (self.0 & 0b0000_0110) >> 1 {
            0 => Bound::None,
            1 => Bound::Exact,
            2 => Bound::Upper,
            3 => Bound::Lower,
            _ => unreachable!(),
        }
    }

    pub const fn pv(&self) -> bool {
        (self.0 & 1) != 0
    }

    pub const fn age(&self) -> u8 {
        self.0 >> 3
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    key: u16,        //2 bytes
    best_move: Move, //2 bytes
    score: i16,      //2 bytes
    eval: i16,       //2 bytes
    depth: u8,       //1 byte
    flags: Flags,    //1 byte
}

impl Entry {
    pub fn new(key: u16, best_move: Move, score: i16, eval: i16, depth: u8, flags: Flags) -> Self {
        Entry {
            key,
            best_move,
            score,
            eval,
            depth,
            flags,
        }
    }

    pub const fn relative_age(&self, tt_age: u8) -> i32 {
        ((32 + tt_age - self.flags.age()) & MAX_AGE) as i32
    }

    pub fn get_key(&self) -> u16 {
        self.key
    }

    pub fn get_bound(&self) -> Bound {
        self.flags.bound()
    }

    pub fn get_best_move(&self) -> Move {
        self.best_move
    }

    pub fn get_score(&self) -> i32 {
        self.score as i32
    }

    pub fn get_depth(&self) -> i32 {
        self.depth as i32
    }

    pub fn get_eval(&self) -> i32 {
        self.eval as i32
    }
}

#[repr(align(32))]
pub struct Cluster {
    entries: [Entry; NUM_ENTRIES_PER_CLUSTER],
}

impl Cluster {
    pub fn lookup_key(&self, key: u16) -> Option<&Entry> {
        self.entries
            .iter()
            .find(|e| e.get_key() == key && e.get_bound() != Bound::None)
    }
}

#[derive(Debug)]
pub struct TranspositionTable {
    clusters: AtomicPtr<Cluster>,
    len: AtomicUsize,
    age: AtomicU8,
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        let (len, p) = unsafe { allocate_entries(size_mb) };
        TranspositionTable {
            clusters: AtomicPtr::new(p),
            len: AtomicUsize::new(len),
            age: AtomicU8::new(0),
        }
    }

    pub fn resize(&self, size_mb: usize) {
        unsafe { deallocate_entries(self.len(), self.ptr()) }
        let (new_len, new_p) = unsafe { allocate_entries(size_mb) };
        self.len.store(new_len, Ordering::Relaxed);
        self.clusters.store(new_p, Ordering::Relaxed);
        self.age.store(0, Ordering::Relaxed);
    }

    fn ptr(&self) -> *mut Cluster {
        self.clusters.load(Ordering::Relaxed)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_entry(
        &self,
        best_move: Move,
        mut score: i32,
        eval: i32,
        bound: Bound,
        hash: u64,
        depth: i32,
        ply: isize,
        pv: bool,
    ) {
        let index = index(hash, self.len());
        debug_assert!(index < self.len());

        let cluster = unsafe { &mut *self.ptr().add(index) };
        let key = hash as u16;
        let tt_age = self.get_age();

        let replacement_index = {
            let mut index = 0;
            let mut worst_quality = i32::MAX;

            for (i, entry) in cluster.entries.iter().enumerate() {
                if entry.get_key() == key || entry.flags.bound() == Bound::None {
                    index = i;
                    break;
                }

                let quality = entry.depth as i32 - 4 * entry.relative_age(tt_age);
                if quality < worst_quality {
                    index = i;
                    worst_quality = quality;
                }
            }

            index
        };

        let entry = &mut cluster.entries[replacement_index];
        let same_key = key == entry.get_key();

        //Keep the stored move if the new move is null for the same position
        if !(same_key && best_move.is_null()) {
            entry.best_move = best_move;
        }

        //Don't replace entry if this is true
        if same_key && depth + 4 + 2 * pv as i32 <= entry.get_depth() && entry.flags.age() == tt_age
        {
            return;
        }

        //Adjust mate scores
        if score.abs() >= Score::MATE_CUTOFF {
            score += score.signum() * ply as i32;
        }

        //Replace entry
        entry.key = key;
        entry.score = score as i16;
        entry.eval = eval as i16;
        entry.depth = depth as u8;
        entry.flags = Flags::new(pv, bound, tt_age);
    }

    pub fn clear(&self) {
        self.age.store(0, Ordering::Relaxed);
        unsafe { self.ptr().write_bytes(0, self.len()) }
    }

    pub fn get_entry(&self, hash: u64) -> Option<Entry> {
        let index = index(hash, self.len());
        debug_assert!(index < self.len());

        let cluster = unsafe { &*self.ptr().add(index) };
        cluster.lookup_key(hash as u16).map(|entry| {
            Entry::new(
                entry.key,
                entry.best_move,
                entry.score,
                entry.eval,
                entry.depth,
                entry.flags,
            )
        })
    }

    pub fn hashfull(&self) -> usize {
        let mut count = 0;
        let clusters = unsafe { std::slice::from_raw_parts(self.ptr(), self.len()) };

        for c in clusters.iter().take(1000) {
            for e in c.entries.iter() {
                if e.flags.bound() != Bound::None && e.flags.age() == self.get_age() {
                    count += 1;
                }
            }
        }

        count / NUM_ENTRIES_PER_CLUSTER
    }

    pub fn get_age(&self) -> u8 {
        self.age.load(Ordering::Relaxed)
    }

    pub fn increase_age(&self) {
        let current_age = self.get_age();
        self.age
            .store((current_age + 1) & MAX_AGE, Ordering::Relaxed);
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.len.load(Ordering::Relaxed)
    }
}

//https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction/
const fn index(hash: u64, len: usize) -> usize {
    (((hash as u128) * (len as u128)) >> 64) as usize
}

impl Default for TranspositionTable {
    fn default() -> Self {
        Self::new(TT_DEFAULT_SIZE)
    }
}

impl Drop for TranspositionTable {
    fn drop(&mut self) {
        unsafe { deallocate_entries(self.len(), self.ptr()) };
    }
}

unsafe fn allocate_entries(size_mb: usize) -> (usize, *mut Cluster) {
    let size = size_mb * MEGABYTE;
    let num_entries = size / SIZE_OF_CLUSTER;

    let layout = std::alloc::Layout::from_size_align(size, align_of::<Cluster>()).unwrap();
    let p = unsafe { std::alloc::alloc_zeroed(layout) };

    (num_entries, p.cast())
}

unsafe fn deallocate_entries(len: usize, p: *mut Cluster) {
    let size = SIZE_OF_CLUSTER * len;
    let layout = std::alloc::Layout::from_size_align(size, align_of::<Cluster>()).unwrap();

    unsafe { std::alloc::dealloc(p.cast(), layout) };
}

#[cfg(test)]
mod tests {
    use crate::{
        board::Board,
        search::{Root, data::SearchData, search},
        types::{Bound, Flags, Score},
    };

    #[test]
    fn test_transposition_table() {
        let board =
            Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - ")
                .unwrap();
        let mut data = SearchData {
            board,
            ..Default::default()
        };

        let score = search::<Root>(&mut data, 3, -Score::INFINITY, Score::INFINITY, 0);

        let hash = data.board.state.hash;
        let entry = data.shared.tt.get_entry(hash).unwrap();

        let m = entry.get_best_move();
        let s = entry.get_score();

        let best_move = data.get_pv().get(0).mv;

        assert_eq!(best_move, m);
        assert_eq!(score, s);

        data.board.make_move(best_move);
        search::<Root>(&mut data, 2, -Score::INFINITY, Score::INFINITY, 0);

        let entry = data.shared.tt.get_entry(hash).unwrap();

        let m = entry.get_best_move();
        let s = entry.get_score();

        assert_eq!(best_move, m);
        assert_eq!(score, s);
    }

    #[test]
    fn test_flags() {
        let flag = Flags::new(true, Bound::Lower, 23);
        println!("{:b}", flag.0);

        assert_eq!(flag.bound(), Bound::Lower);
        assert!(flag.pv());
        assert_eq!(23, flag.age());
    }
}
