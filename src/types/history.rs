use crate::types::{BitBoard, Move, Piece, Side, Square, to_piece_index};

pub type FromToHistory<T> = [[T; 64]; 64];
pub type PieceToHistory<T> = [[T; 64]; 13];

#[derive(Debug, Clone)]
//[Side to Move][From Threatened][To Threatened][From][To]
pub struct QuietHistory(Box<[[[FromToHistory<i16>; 2]; 2]; 2]>);

impl QuietHistory {
    const MAX_HISTORY: i32 = 8199;

    pub fn new() -> Self {
        Self(allocate_empty_history())
    }

    pub fn update(&mut self, threats: BitBoard, side: Side, m: Move, bonus: i32) {
        let from = m.get_from();
        let to = m.get_to();

        let from_threats = threats.contains(from);
        let to_threats = threats.contains(to);

        let entry = &mut self.0[side as usize][from_threats as usize][to_threats as usize]
            [from as usize][to as usize];
        update_entry::<{ Self::MAX_HISTORY }>(bonus, entry);
    }

    pub fn get(&self, threats: BitBoard, side: Side, m: Move) -> i32 {
        let from = m.get_from();
        let to = m.get_to();

        let from_threats = threats.contains(from);
        let to_threats = threats.contains(to);

        self.0[side as usize][from_threats as usize][to_threats as usize][from as usize]
            [to as usize] as i32
    }
}

#[derive(Debug, Clone)]
//[Piece][To][Captured Piece][To Threatened]
pub struct NoisyHistory(Box<PieceToHistory<[[i16; 2]; 7]>>);

impl NoisyHistory {
    const MAX_HISTORY: i32 = 8113;

    pub fn new() -> Self {
        Self(allocate_empty_history())
    }

    pub fn update(
        &mut self,
        piece: Option<(Side, Piece)>,
        to: Square,
        captured: Option<Piece>,
        threats: BitBoard,
        bonus: i32,
    ) {
        let piece_index = match piece {
            Some((s, p)) => (s as usize * 6) + p as usize,
            None => 12,
        };

        let captured_index = match captured {
            Some(p) => p as usize,
            None => 6,
        };

        let entry =
            &mut self.0[piece_index][to as usize][captured_index][threats.contains(to) as usize];
        update_entry::<{ Self::MAX_HISTORY }>(bonus, entry);
    }

    pub fn get(
        &self,
        piece: Option<(Side, Piece)>,
        to: Square,
        captured: Option<Piece>,
        threats: BitBoard,
    ) -> i32 {
        let piece_index = match piece {
            Some((s, p)) => (s as usize * 6) + p as usize,
            None => 12,
        };

        let captured_index = match captured {
            Some(p) => p as usize,
            None => 6,
        };

        self.0[piece_index][to as usize][captured_index][threats.contains(to) as usize] as i32
    }
}

#[derive(Debug, Clone)]
//[Piece][To][Piece][To]
pub struct ContinuationHistory(Box<PieceToHistory<PieceToHistory<i16>>>);

impl ContinuationHistory {
    pub const MAX_HISTORY: i32 = 7890;

    pub fn new() -> Self {
        Self(allocate_empty_history())
    }

    pub fn subtable(
        &mut self,
        piece: Option<(Side, Piece)>,
        to: Square,
    ) -> *mut PieceToHistory<i16> {
        &raw mut self.0[to_piece_index(piece)][to as usize]
    }

    /// # Safety
    /// 'subtable' needs to point to a valid subtable owned by the history
    pub unsafe fn update(
        &mut self,
        subtable: *mut PieceToHistory<i16>,
        piece: Option<(Side, Piece)>,
        to: Square,
        bonus: i32,
    ) {
        let entry = &mut unsafe { &mut *subtable }[to_piece_index(piece)][to as usize];
        update_entry::<{ Self::MAX_HISTORY }>(bonus, entry);
    }

    /// # Safety
    /// 'subtable' needs to point to a valid subtable owned by the history
    pub unsafe fn get(
        &self,
        subtable: *mut PieceToHistory<i16>,
        piece: Option<(Side, Piece)>,
        to: Square,
    ) -> i32 {
        (unsafe { &*subtable }[to_piece_index(piece)][to as usize]) as i32
    }
}

#[derive(Debug, Clone)]
//[Side to Move][Key]
pub struct CorrectionHistory(Box<[[i16; Self::SIZE]; 2]>);

impl CorrectionHistory {
    const MAX_HISTORY: i32 = 12000;

    const SIZE: usize = 65536;
    const MASK: usize = Self::SIZE - 1;

    pub fn new() -> Self {
        Self(allocate_empty_history())
    }

    pub fn update(&mut self, stm: Side, key: u64, bonus: i32) {
        let entry = &mut self.0[stm as usize][key as usize & Self::MASK];
        update_entry::<{ Self::MAX_HISTORY }>(bonus, entry);
    }

    pub fn get(&self, stm: Side, key: u64) -> i32 {
        self.0[stm as usize][key as usize & Self::MASK] as i32
    }
}

fn allocate_empty_history<T>() -> Box<T> {
    let layout = std::alloc::Layout::new::<T>();
    unsafe {
        let p = std::alloc::alloc_zeroed(layout);
        Box::<T>::from_raw(p.cast())
    }
}

fn update_entry<const MAX: i32>(bonus: i32, entry: &mut i16) {
    let clamped_bonus = bonus.clamp(-MAX, MAX);
    *entry += (clamped_bonus - (*entry as i32) * clamped_bonus.abs() / MAX) as i16;
}

impl Default for QuietHistory {
    fn default() -> Self {
        QuietHistory::new()
    }
}

impl Default for NoisyHistory {
    fn default() -> Self {
        NoisyHistory::new()
    }
}

impl Default for ContinuationHistory {
    fn default() -> Self {
        ContinuationHistory::new()
    }
}

impl Default for CorrectionHistory {
    fn default() -> Self {
        CorrectionHistory::new()
    }
}

#[cfg(test)]
pub mod tests {
    use crate::types::{BitBoard, NoisyHistory, Piece, Side, Square};

    #[test]
    fn test_history() {
        let mut noisy_history = NoisyHistory::new();

        let entry = noisy_history.get(None, Square::A4, None, BitBoard(0));
        println!("{}", entry);
        let piece = Some((Side::Black, Piece::Bishop));
        let captured = Some(Piece::Queen);
        noisy_history.update(piece, Square::A4, captured, BitBoard(0), 32);
        let entry2 = noisy_history.get(piece, Square::A4, captured, BitBoard(0));
        let entry = noisy_history.get(None, Square::A4, None, BitBoard(0));

        assert_eq!(entry2, 32);
        assert_eq!(entry, 0);
    }
}
