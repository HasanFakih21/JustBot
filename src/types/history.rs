use crate::{
    tools::parameters::{max_cont_history, max_corr_history, max_noisy_history, max_quiet_history},
    types::{BitBoard, Move, Piece, Side, Square, to_piece_index},
};

pub type FromToHistory<T> = [[T; 64]; 64];
pub type PieceToHistory<T> = [[T; 64]; 13];

#[derive(Debug, Clone)]
// [Side to Move][From Threatened][To Threatened][From][To]
pub struct QuietHistory(Box<[[[FromToHistory<i16>; 2]; 2]; 2]>);

impl QuietHistory {
    pub fn new() -> Self {
        unsafe { Self(allocate_zeroed_box()) }
    }

    pub fn update(&mut self, threats: BitBoard, side: Side, m: Move, bonus: i32) {
        let from = m.get_from();
        let to = m.get_to();

        let from_threats = threats.contains(from);
        let to_threats = threats.contains(to);

        let entry = &mut self.0[side as usize][from_threats as usize][to_threats as usize]
            [from as usize][to as usize];
        update_entry(bonus, entry, max_quiet_history());
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
// [Piece][To][Captured Piece][To Threatened]
pub struct NoisyHistory(Box<PieceToHistory<[[i16; 2]; 7]>>);

impl NoisyHistory {
    pub fn new() -> Self {
        unsafe { Self(allocate_zeroed_box()) }
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
        update_entry(bonus, entry, max_noisy_history());
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
// [Piece][To][Piece][To]
pub struct ContinuationHistory(Box<PieceToHistory<PieceToHistory<i16>>>);

impl ContinuationHistory {
    pub fn new() -> Self {
        unsafe { Self(allocate_zeroed_box()) }
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
        update_entry(bonus, entry, max_cont_history());
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
// [Side to Move][Key]
pub struct CorrectionHistory(Box<[[i16; Self::SIZE]; 2]>);

impl CorrectionHistory {
    const SIZE: usize = 16384;
    const MASK: usize = Self::SIZE - 1;

    pub fn new() -> Self {
        unsafe { Self(allocate_zeroed_box()) }
    }

    pub fn update(&mut self, stm: Side, key: u64, bonus: i32) {
        let entry = &mut self.0[stm as usize][key as usize & Self::MASK];
        update_entry(bonus, entry, max_corr_history());
    }

    pub fn get(&self, stm: Side, key: u64) -> i32 {
        self.0[stm as usize][key as usize & Self::MASK] as i32
    }
}

#[derive(Debug, Clone)]
// [Pawn Key][Piece][To]
pub struct PawnHistory(Box<[PieceToHistory<i16>; Self::SIZE]>);

impl PawnHistory {
    const SIZE: usize = 512;
    const MASK: usize = Self::SIZE - 1;

    pub fn new() -> Self {
        unsafe { Self(allocate_zeroed_box()) }
    }

    pub fn update(&mut self, key: u64, piece: Option<(Side, Piece)>, to: Square, bonus: i32) {
        let entry = &mut self.0[key as usize & Self::MASK][to_piece_index(piece)][to];
        update_entry(bonus, entry, 8000);
    }

    pub fn get(&self, key: u64, piece: Option<(Side, Piece)>, to: Square) -> i32 {
        self.0[key as usize & Self::MASK][to_piece_index(piece)][to] as i32
    }
}

/// # Safety
/// The type 'T' needs to be able to be zero-initialized
pub unsafe fn allocate_zeroed_box<T>() -> Box<T> {
    let layout = std::alloc::Layout::new::<T>();
    let p = unsafe { std::alloc::alloc_zeroed(layout) };
    if p.is_null() {
        std::alloc::handle_alloc_error(layout);
    }

    unsafe { Box::<T>::from_raw(p.cast()) }
}

fn update_entry(bonus: i32, entry: &mut i16, max: i32) {
    let clamped_bonus = bonus.clamp(-max, max);
    *entry += (clamped_bonus - (*entry as i32) * clamped_bonus.abs() / max) as i16;
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

impl Default for PawnHistory {
    fn default() -> Self {
        PawnHistory::new()
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
