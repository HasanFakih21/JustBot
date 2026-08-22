use std::sync::Mutex;

use crate::attacks::*;
use crate::types::*;

static SEED: Mutex<u32> = Mutex::new(1804289383);

pub static BISHOP_MAGIC_NUMBERS: [u64; 64] = [
    0x40040844404084,
    0x2004208a004208,
    0x10190041080202,
    0x108060845042010,
    0x581104180800210,
    0x2112080446200010,
    0x1080820820060210,
    0x3c0808410220200,
    0x4050404440404,
    0x21001420088,
    0x24d0080801082102,
    0x1020a0a020400,
    0x40308200402,
    0x4011002100800,
    0x401484104104005,
    0x801010402020200,
    0x400210c3880100,
    0x404022024108200,
    0x810018200204102,
    0x4002801a02003,
    0x85040820080400,
    0x810102c808880400,
    0xe900410884800,
    0x8002020480840102,
    0x220200865090201,
    0x2010100a02021202,
    0x152048408022401,
    0x20080002081110,
    0x4001001021004000,
    0x800040400a011002,
    0xe4004081011002,
    0x1c004001012080,
    0x8004200962a00220,
    0x8422100208500202,
    0x2000402200300c08,
    0x8646020080080080,
    0x80020a0200100808,
    0x2010004880111000,
    0x623000a080011400,
    0x42008c0340209202,
    0x209188240001000,
    0x400408a884001800,
    0x110400a6080400,
    0x1840060a44020800,
    0x90080104000041,
    0x201011000808101,
    0x1a2208080504f080,
    0x8012020600211212,
    0x500861011240000,
    0x180806108200800,
    0x4000020e01040044,
    0x300000261044000a,
    0x802241102020002,
    0x20906061210001,
    0x5a84841004010310,
    0x4010801011c04,
    0xa010109502200,
    0x4a02012000,
    0x500201010098b028,
    0x8040002811040900,
    0x28000010020204,
    0x6000020202d0240,
    0x8918844842082200,
    0x4010011029020020,
];

pub static ROOK_MAGIC_NUMBERS: [u64; 64] = [
    0x8a80104000800020,
    0x140002000100040,
    0x2801880a0017001,
    0x100081001000420,
    0x200020010080420,
    0x3001c0002010008,
    0x8480008002000100,
    0x2080088004402900,
    0x800098204000,
    0x2024401000200040,
    0x100802000801000,
    0x120800800801000,
    0x208808088000400,
    0x2802200800400,
    0x2200800100020080,
    0x801000060821100,
    0x80044006422000,
    0x100808020004000,
    0x12108a0010204200,
    0x140848010000802,
    0x481828014002800,
    0x8094004002004100,
    0x4010040010010802,
    0x20008806104,
    0x100400080208000,
    0x2040002120081000,
    0x21200680100081,
    0x20100080080080,
    0x2000a00200410,
    0x20080800400,
    0x80088400100102,
    0x80004600042881,
    0x4040008040800020,
    0x440003000200801,
    0x4200011004500,
    0x188020010100100,
    0x14800401802800,
    0x2080040080800200,
    0x124080204001001,
    0x200046502000484,
    0x480400080088020,
    0x1000422010034000,
    0x30200100110040,
    0x100021010009,
    0x2002080100110004,
    0x202008004008002,
    0x20020004010100,
    0x2048440040820001,
    0x101002200408200,
    0x40802000401080,
    0x4008142004410100,
    0x2060820c0120200,
    0x1001004080100,
    0x20c020080040080,
    0x2935610830022400,
    0x44440041009200,
    0x280001040802101,
    0x2100190040002085,
    0x80c0084100102001,
    0x4024081001000421,
    0x20030a0244872,
    0x12001008414402,
    0x2006104900a0804,
    0x1004081002402,
];

// XOR Shift Pseudo-Random Number Generator
pub fn random_num() -> u32 {
    let mut number = SEED.lock().unwrap();

    *number ^= *number << 13;
    *number ^= *number >> 17;
    *number ^= *number << 5;

    *number
}

pub fn random_u64_num() -> u64 {
    let nums: [u64; 4] = std::array::from_fn(|_e| (random_num() as u64) & 0xFFFF);

    nums[0] | (nums[1] << 16) | (nums[2] << 32) | (nums[3] << 48)
}

pub fn generate_magic_number() -> u64 {
    random_u64_num() & random_u64_num() & random_u64_num()
}

pub fn find_magic_number(square: Square, piece: Piece) -> u64 {
    let mut occupancies: [BitBoard; 4096] = [BitBoard(0); 4096];
    let mut attacks: [BitBoard; 4096] = [BitBoard(0); 4096];
    let mut used_attacks: [BitBoard; 4096];

    let attack_mask: BitBoard = match piece {
        Piece::Bishop => mask_bishop_attacks(square),
        Piece::Rook => mask_rook_attacks(square),
        _ => return 0,
    };

    let relevant_bits = match piece {
        Piece::Bishop => BISHOP_OCCUPANCY_BIT_COUNTS[square as usize],
        Piece::Rook => ROOK_OCCUPANCY_BIT_COUNTS[square as usize],
        _ => return 0,
    };

    for index in 0..(1 << relevant_bits) {
        occupancies[index] = set_occupancy(index, relevant_bits, attack_mask);

        attacks[index] = match piece {
            Piece::Bishop => blocked_bishop_attacks(square, occupancies[index]),
            Piece::Rook => blocked_rook_attacks(square, occupancies[index]),
            _ => return 0,
        }
    }

    for _ in 0..10000000 {
        let magic_number = generate_magic_number();

        if ((attack_mask.0.wrapping_mul(magic_number)) & 0xFF00000000000000).count_ones() < 6 {
            continue;
        }

        used_attacks = [BitBoard(0); 4096];

        let (mut index, mut fail) = (0, false);
        while !fail && (index < (1 << relevant_bits)) {
            let magic_index = magic_index(occupancies[index], relevant_bits, magic_number);

            if used_attacks[magic_index] == BitBoard(0) {
                used_attacks[magic_index] = attacks[index];
            } else if used_attacks[magic_index] != attacks[index] {
                fail = true;
            }
            index += 1;
        }

        if !fail {
            return magic_number;
        }
    }

    println!("Magic number generation failed");
    0
}

pub const fn magic_index(occ_bb: BitBoard, relevant_bits: usize, magic_number: u64) -> usize {
    ((occ_bb.0.wrapping_mul(magic_number)) >> (64 - relevant_bits)) as usize
}

#[rustfmt::skip]
pub const BISHOP_OCCUPANCY_BIT_COUNTS: [usize; 64] = [
    6, 5, 5, 5, 5, 5, 5, 6,
    5, 5, 5, 5, 5, 5, 5, 5,
    5, 5, 7, 7, 7, 7, 5, 5,
    5, 5, 7, 9, 9, 7, 5, 5,
    5, 5, 7, 9, 9, 7, 5, 5,
    5, 5, 7, 7, 7, 7, 5, 5,
    5, 5, 5, 5, 5, 5, 5, 5,
    6, 5, 5, 5, 5, 5, 5, 6,
];

#[rustfmt::skip]
pub const ROOK_OCCUPANCY_BIT_COUNTS: [usize; 64] = [
    12, 11, 11, 11, 11, 11, 11, 12,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    12, 11, 11, 11, 11, 11, 11, 12,
];

pub fn set_occupancy(index: usize, num_bits_in_mask: usize, mut attack_mask: BitBoard) -> BitBoard {
    let mut occupancy = BitBoard(0u64);

    for count in 0..num_bits_in_mask {
        let square = attack_mask.least_sig_bit().unwrap();
        attack_mask.clear_bit(square);

        if (index & (1 << count)) != 0 {
            occupancy.0 |= 1u64 << square as usize;
        }
    }

    occupancy
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    //     fn magic_test() {
    // for i in 0..64 {
    // let square = Square::from(i);
    // println!("0x{:x}, ", find_magic_number(square, Piece::Rook));
    // }

    // println!();

    // for i in 0..64 {
    // let square = Square::from(i);
    // println!("0x{:x}, ", find_magic_number(square, Piece::Bishop));
    // }
    // }

    #[test]
    fn test_random_num() {
        println!("{}", random_num());
        println!("{}", random_num());
        println!("{}", random_num());
        println!("{}", random_num());
    }

    #[test]
    fn test_u64_random_num() {
        println!("{}", random_u64_num());
        println!("{}", random_u64_num());
        println!("{}", random_u64_num());
        println!("{}", random_u64_num());
        println!("{}", random_u64_num());
    }

    #[test]
    fn test_set_occupancy() {
        let attack_mask = mask_rook_attacks(Square::A1);
        for i in 0..=4096 {
            let occupancy_bb = set_occupancy(i, 12, attack_mask);
            occupancy_bb.print_board();
        }
    }
}
