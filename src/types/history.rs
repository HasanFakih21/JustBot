use crate::types::{BitBoard, Move, OptionPiece, Piece, Side, SidedPiece, Square};

pub type FromToHistory<T> = [[T; 64]; 64];
pub type PieceToHistory<T> = [[T; 64]; 13];

#[derive(Debug, Clone)]
// [Side to Move][From Threatened][To Threatened][From][To]
pub struct QuietHistory(Box<[[[FromToHistory<i16>; 2]; 2]; 2]>);

impl QuietHistory {
    const MAX_HISTORY: i32 = 8128;

    pub fn new() -> Self {
        Self(zeroed_box())
    }

    pub fn update(&mut self, threats: BitBoard, side: Side, m: Move, bonus: i32) {
        let from = m.from();
        let to = m.to();

        let from_threats = threats.contains(from);
        let to_threats = threats.contains(to);

        let entry = &mut self.0[side as usize][from_threats as usize][to_threats as usize]
            [from as usize][to as usize];
        update_entry::<{ Self::MAX_HISTORY }>(bonus, entry);
    }

    pub fn get(&self, threats: BitBoard, side: Side, m: Move) -> i32 {
        let from = m.from();
        let to = m.to();

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
    const MAX_HISTORY: i32 = 8209;

    pub fn new() -> Self {
        Self(zeroed_box())
    }

    pub fn update(
        &mut self,
        piece: OptionPiece<SidedPiece>,
        to: Square,
        captured: OptionPiece<Piece>,
        threats: BitBoard,
        bonus: i32,
    ) {
        let entry = &mut self.0[piece][to as usize][captured][threats.contains(to) as usize];
        update_entry::<{ Self::MAX_HISTORY }>(bonus, entry);
    }

    pub fn get(
        &self,
        piece: OptionPiece<SidedPiece>,
        to: Square,
        captured: OptionPiece<Piece>,
        threats: BitBoard,
    ) -> i32 {
        self.0[piece][to as usize][captured][threats.contains(to) as usize] as i32
    }
}

#[derive(Debug, Clone)]
// [Piece][To][Piece][To]
pub struct ContinuationHistory(Box<PieceToHistory<PieceToHistory<i16>>>);

impl ContinuationHistory {
    pub const MAX_HISTORY: i32 = 7813;

    pub fn new() -> Self {
        Self(zeroed_box())
    }

    pub fn subtable(
        &mut self,
        piece: OptionPiece<SidedPiece>,
        to: Square,
    ) -> *mut PieceToHistory<i16> {
        &raw mut self.0[piece][to as usize]
    }

    /// # Safety
    /// 'subtable' needs to point to a valid subtable owned by the history
    pub unsafe fn update(
        &mut self,
        subtable: *mut PieceToHistory<i16>,
        piece: OptionPiece<SidedPiece>,
        to: Square,
        bonus: i32,
    ) {
        let entry = &mut unsafe { &mut *subtable }[piece][to as usize];
        update_entry::<{ Self::MAX_HISTORY }>(bonus, entry);
    }

    /// # Safety
    /// 'subtable' needs to point to a valid subtable owned by the history
    pub unsafe fn get(
        &self,
        subtable: *mut PieceToHistory<i16>,
        piece: OptionPiece<SidedPiece>,
        to: Square,
    ) -> i32 {
        (unsafe { &*subtable }[piece][to as usize]) as i32
    }
}

#[derive(Debug, Clone)]
// [Piece][To][Piece][To]
pub struct ContinuationCorrectionHistory(Box<PieceToHistory<PieceToHistory<i16>>>);

impl ContinuationCorrectionHistory {
    pub const MAX_HISTORY: i32 = 12000;

    pub fn new() -> Self {
        Self(zeroed_box())
    }

    pub fn subtable(
        &mut self,
        piece: OptionPiece<SidedPiece>,
        to: Square,
    ) -> *mut PieceToHistory<i16> {
        &raw mut self.0[piece][to as usize]
    }

    /// # Safety
    /// 'subtable' needs to point to a valid subtable owned by the history
    pub unsafe fn update(
        &mut self,
        subtable: *mut PieceToHistory<i16>,
        piece: OptionPiece<SidedPiece>,
        to: Square,
        bonus: i32,
    ) {
        let entry = &mut unsafe { &mut *subtable }[piece][to as usize];
        update_entry::<{ Self::MAX_HISTORY }>(bonus, entry);
    }

    /// # Safety
    /// 'subtable' needs to point to a valid subtable owned by the history
    pub unsafe fn get(
        &self,
        subtable: *mut PieceToHistory<i16>,
        piece: OptionPiece<SidedPiece>,
        to: Square,
    ) -> i32 {
        (unsafe { &*subtable }[piece][to as usize]) as i32
    }
}

#[derive(Debug, Clone)]
// [Side to Move][Key]
pub struct CorrectionHistory(Box<[[i16; Self::SIZE]; 2]>);

impl CorrectionHistory {
    const MAX_HISTORY: i32 = 11972;

    const SIZE: usize = 16384;
    const MASK: usize = Self::SIZE - 1;

    pub fn new() -> Self {
        Self(zeroed_box())
    }

    pub fn update(&mut self, stm: Side, key: u64, bonus: i32) {
        let entry = &mut self.0[stm as usize][key as usize & Self::MASK];
        update_entry::<{ Self::MAX_HISTORY }>(bonus, entry);
    }

    pub fn get(&self, stm: Side, key: u64) -> i32 {
        self.0[stm as usize][key as usize & Self::MASK] as i32
    }
}

#[derive(Debug, Clone)]
// [Pawn Key][Piece][To]
pub struct PawnHistory(Box<[PieceToHistory<i16>; Self::SIZE]>);

impl PawnHistory {
    const MAX_HISTORY: i32 = 8000;

    const SIZE: usize = 512;
    const MASK: usize = Self::SIZE - 1;

    pub fn new() -> Self {
        Self(zeroed_box())
    }

    pub fn update(&mut self, key: u64, piece: OptionPiece<SidedPiece>, to: Square, bonus: i32) {
        let entry = &mut self.0[key as usize & Self::MASK][piece][to];
        update_entry::<{ Self::MAX_HISTORY }>(bonus, entry);
    }

    pub fn get(&self, key: u64, piece: OptionPiece<SidedPiece>, to: Square) -> i32 {
        self.0[key as usize & Self::MASK][piece][to] as i32
    }
}

pub fn zeroed_box<T>() -> Box<T> {
    let layout = std::alloc::Layout::new::<T>();
    unsafe {
        let p = std::alloc::alloc_zeroed(layout);
        if p.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
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

impl Default for ContinuationCorrectionHistory {
    fn default() -> Self {
        ContinuationCorrectionHistory::new()
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
