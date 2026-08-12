// Code is originally from akimbo: https://github.com/jw1912/akimbo under the MIT license
// Modifications from Hobbes: https://github.com/kelseyde/hobbes-chess-engine

#[cfg(feature = "tuning")]
#[macro_export]
macro_rules! tunable_params {
    ($($name:ident = $val:expr, $min:literal ..= $max:literal, $spsa:expr;)*) => {
        use std::sync::atomic::Ordering;

        pub fn list_params() {
            $(
                println!(
                    "option name {} type spin default {} min {} max {}",
                    stringify!($name),
                    $name(),
                    $min,
                    $max,
                );
            )*
        }

        pub fn set_param(name: &str, val: i32) {
            match name {
                $(
                    stringify!($name) => vals::$name.store(val, Ordering::Relaxed),
                )*
                _ => println!("info error unknown option"),
            }
        }

        pub fn print_params_ob() {
            $(
                if $spsa {
                    let step = (($max - $min) / 20).max(1);
                    println!(
                        "{}, int, {}.0, {}.0, {}.0, {}, 0.002",
                        stringify!($name),
                        $name(),
                        $min,
                        $max,
                        step,
                    );
                }
            )*
        }

        mod vals {
            use std::sync::atomic::AtomicI32;
            $(
            #[allow(non_upper_case_globals)]
            pub static $name: AtomicI32 = AtomicI32::new($val);
            )*
        }

        $(
        #[inline]
        pub fn $name() -> i32 {
            vals::$name.load(Ordering::Relaxed)
        }
        )*
    };
}

#[rustfmt::skip]
tunable_params!{
    // Aspiration Windows
    asp_init_alpha_window = 33, 15..=100, false;
    asp_init_beta_window  = 20, 15..=100, false;
    asp_alpha_window      = 30, 15..=100, false;
    asp_beta_window       = 19, 15..=100, false;

    // Base Time Scaling
    tm_base    = 60, 50..=75, false;
    tm_mult    = 50, 30..=65, false;
    tm_fm_mult = 35, 20..=50, false;

    // Node Time Scaling
    node_tm_base = 2977, 2500..=3500, false;
    node_tm_mult = 2495, 2000..=3000, false;
    node_tm_min  = 553,  450..=750,   false;

    // Razoring
    razoring_base = 246, 100..=400, false;
    razoring_mult = 253, 100..=400, false;

    // RFP
    rfp_base      = 147, 100..=300, false;
    rfp_improving = 94,  50..=150,  false;
    rfp_lerp_t    = 690, 300..=900, false;

    // NMP
    nmp_depth     = 3,   2..=4,     false;
    nmp_improving = 63,  30..=150,  false;
    nmp_base_r    = 6,   3..=9,     false;
    nmp_r_mult    = 132, 100..=300, false;
    nmp_r_div     = 637, 400..=900, false;

    // SE
    se_depth       = 5,   4..=8,     false;
    se_tt_depth    = 3,   1..=5,     false;
    se_double_base = 10,  5..=50,    false;
    se_double_pv   = 252, 100..=400, false;
    se_neg_ext     = 2,   1..=4,     false;

    // LMP
    lmp_base = 3011, 2000..=5000, false;
    lmp_mult = 1493, 1000..=3000, false;

    // FP
    fp_depth    = 8, 4..=9,       false;
    fp_mult     = 93, 50..=150,   false;
    fp_base     = 146, 100..=200, false;
    fp_history  = 50, 5..=200,    false;

    // HP
    hp_depth = 6,    3..=8,       false;
    hp_base  = 1485, 1000..=2000, false;

    // Main SEE
    see_mult1 = 125, 100..=250, false;
    see_mult2 = 46,  20..=150,  false;
    see_base  = 14,  5..=50,    false;
    see_max   = 34,  5..=100,   false;

    // Base LMR
    lmr_base      = 7851,  5000..=12000,  false;
    lmr_quiet_div = 24482, 20000..=32000, false;
    lmr_noisy_div = 30040, 25000..=36000, false;

    // LMR
    lmr_depth     = 2, 1..=3,       false;
    lmr_improving = 217, 150..=400, false;
    lmr_ttpv      = 197, 100..=500, false;
    lmr_tt_alpha  = 447, 100..=600, false;
    lmr_tt_depth  = 296, 150..=600, false;
    lmr_history   = 449, 150..=800, false;

    // Quiet History
    hist_quiet_bonus_mult = 321, 150..=500,   false;
    hist_quiet_bonus_max  = 935, 800..=1400,  false;
    hist_quiet_bonus_base = 228, 150..=400,   false;
    hist_quiet_malus_mult = 289, 150..=500,   false;
    hist_quiet_malus_max  = 948, 800..=1400,  false;
    hist_quiet_malus_base = 232, 150..=400,   false;

    // Noisy History
    hist_noisy_bonus_mult = 257, 150..=500,   false;
    hist_noisy_bonus_max  = 1058, 800..=1400, false;
    hist_noisy_bonus_base = 196, 150..=400,   false;
    hist_noisy_malus_mult = 302, 150..=500,   false;
    hist_noisy_malus_max  = 937, 800..=1400,  false;
    hist_noisy_malus_base = 273, 150..=400,   false;

    // Continuation History
    hist_cont_bonus_mult = 315, 150..=500,    false;
    hist_cont_bonus_max  = 1044, 800..=1400,  false;
    hist_cont_bonus_base = 194, 150..=400,    false;
    hist_cont_malus_mult = 303, 150..=500,    false;
    hist_cont_malus_max  = 1079, 800..=1400,  false;
    hist_cont_malus_base = 271, 150..=400,    false;

    // Correction History
    corr_hist_base = 148, 50..=400,        false;
    corr_hist_div  = 121, 80..=160,        false;
    corr_hist_min  = -4612, -6000..=-2000, false;
    corr_hist_max  = 2530, 1000..=5000,    false;
    corr_div       = 64, 50..=90,          false;

    // Qsearch
    qsearch_lmp        =  3,    2..=5,     false;
    qsearch_see        = -129, -400..=150, false;
    qsearch_hist_bonus =  103,   50..=300,  false;

    // SEE
    see_pawn   = 88,   50..=150,    false;
    see_knight = 428,  350..=450,   false;
    see_bishop = 458,  400..=550,   false;
    see_rook   = 659,  600..=800,   false;
    see_queen  = 1289, 1000..=1500, false;

    // Move Picker
    mp_see_base = 64, 40..=100, false; 
    mp_see_div  = 4,  3..=6,    false;

    // Score Noisy
    score_noisy_div = 8, 5..=10, false;

    // Score Quiet
    score_quiet_pawn   = 1000, 500..=1500,   false;
    score_quiet_cont1  = 1595, 1200..=2000,  false;
    score_quiet_cont2  = 1050, 800..=1600,   false;
    direct_check_bonus = 9808, 9000..=12000, false; 

    // Max Histories
    max_quiet_history = 8128,  7500..=15000,  false;
    max_noisy_history = 8209,  7500..=15000,  false;
    max_cont_history  = 7813,  7500..=15000,  false;
    max_corr_history  = 11972, 7500..=15000,  false;
    max_pawn_history  = 8000,  7500..=15000,  false;

    // Material Scaling
    mat_scale_pawn   = 50,    50..=150,       true;
    mat_scale_knight = 420,   350..=450,      true;
    mat_scale_bishop = 458,   400..=550,      true;
    mat_scale_rook   = 650,   600..=800,      true;
    mat_scale_queen  = 1200,  1000..=1500,    true;
    mat_scale_base   = 26000, 10000..=40000,  true;

    // Piece Values
    value_pawn   = 100,    50..=150,     false;
    value_knight = 320,   250..=325,     false;
    value_bishop = 330,   325..=450,     false;
    value_rook   = 500,   460..=800,     false;
    value_queen  = 900,   850..=1500,    false;
}
