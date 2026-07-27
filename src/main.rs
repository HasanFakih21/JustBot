use justbot::{
    attacks::{BISHOP_ATTACKS, ROOK_ATTACKS},
    types::LMR_TABLE,
};
use std::sync::LazyLock;

fn main() {
    // Just to warm up the lazy locks
    LazyLock::force(&ROOK_ATTACKS);
    LazyLock::force(&BISHOP_ATTACKS);
    LazyLock::force(&LMR_TABLE);

    let args: Vec<String> = std::env::args().skip(1).collect();
    justbot::tools::uci::input_loop(args.join(" "));
}
