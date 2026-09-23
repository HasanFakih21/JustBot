// Code is originally from akimbo: https://github.com/jw1912/akimbo under the MIT license
// Modifications from Hobbes: https://github.com/kelseyde/hobbes-chess-engine

#[macro_export]
macro_rules! tunable_params {
    ($($name:ident = $val:expr, $min:literal ..= $max:literal, $spsa:expr;)*) => {
        #[allow(unused)]
        #[cfg(feature = "tuning")]
        use std::sync::atomic::Ordering;

        #[cfg(feature = "tuning")]
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

        #[allow(unused)]
        #[cfg(feature = "tuning")]
        pub fn set_param(name: &str, val: i32) {
            match name {
                $(
                    stringify!($name) => vals::$name.store(val, Ordering::Relaxed),
                )*
                _ => println!("info error unknown option"),
            }
        }

        #[cfg(feature = "tuning")]
        pub fn print_params_ob() {
            $(
                if $spsa {
                    let c_end = (($max - $min) as f32 / 20.0);
                    let r_end = 0.002 / (c_end.min(0.5) / 0.5);
                    println!(
                        "{}, int, {}.0, {}.0, {}.0, {}, {}",
                        stringify!($name),
                        $name(),
                        $min,
                        $max,
                        c_end,
                        r_end,
                    );
                }
            )*
        }

        #[cfg(feature = "tuning")]
        mod vals {
            #[allow(unused)]
            use std::sync::atomic::AtomicI32;
            $(
            #[allow(non_upper_case_globals)]
            pub static $name: AtomicI32 = AtomicI32::new($val);
            )*
        }

        $(
        #[cfg(feature = "tuning")]
        #[inline]
        pub fn $name() -> i32 {
            vals::$name.load(Ordering::Relaxed)
        }

        #[cfg(not(feature = "tuning"))]
        #[inline]
        pub const fn $name() -> i32 {
            $val
        }
        )*
    };
}

#[rustfmt::skip]
tunable_params! {
    // Aspiration Window
    init_delta = 25, 5..=50, true;
    fail_low_delta = 24, 5..=50, true;
    fail_high_delta = 25, 5..=50, true;

    // TT Cutoffs
    tt_cutoff_quiet_scale = 206, 50..=500, true;
    tt_cutoff_quiet_offset = 50, 5..=150, true;
    tt_cutoff_quiet_max = 1201, 800..=1800, true;

    // Hindsight
    hindsight_ext_reduction = 3093, 2100..=4600, true;
    hindsight_red_reduction = 2078, 1024..=3048, true;
    hindsight_red_eval = 211, 5..=500, true;

    // Razoring
    razoring_offset = 242, 100..=400, true;
    razoring_scale = 254, 100..=400, true;

    // RFP
    rfp_scale_1 = 87, 40..=140, true;
    rfp_scale_2 = 6, 1..=20, true;
    rfp_improving = 73, 40..=150, true;
    rfp_t = 686, 500..=900, true;

    // NMP
    nmp_offset = 199, 100..=400, true;
    nmp_scale = 1214, 800..=1500, true;
    nmp_improving = 64, 35..=100, true;
    nmp_r_scale = 124, 40..=300, true;

    // ProbCut
    probcut_margin = 250, 100..=400, true;

    // Singular Extensions
    se_double_base = 10, 5..=30, true;
    se_double_pv = 150, 50..=300, true;
    se_double_new_pv = 50, 10..=100, true;
    se_triple_base = 100, 50..=200, true;
    se_triple_pv = 351, 250..=450, true;
    se_triple_new_pv = 55, 10..=200, true;

    // LMP
    lmp_base1 = 2976, 2000..=4500, true;
    lmp_base2 = 1363, 500..=2500, true;
    lmp_improving = 263, 150..=450, true;

    // FP
    fp_scale = 93, 40..=200, true;
    fp_offset = 142, 50..=350, true;
    fp_history = 51, 15..=150, true;

    // HP
    hp_scale = -1481, -2000..=-500, true;

    // Main Search SEE Pruning
    main_see_scale = -123, -400..=-20, true;
    main_see_scale2 = 44, 25..=150, true;
    main_see_offset = 14, 5..=50, true;
    main_see_max = -35, -100..=0, true;

    // LMR
    lmr_quiet_base = 5602, 2550..=8550, true;
    lmr_noisy_base = 1085, -3050..=4050, true;

    lmr_quiet_div = 24566, 20000..=28500, true;
    lmr_noisy_div = 30218, 25000..=35000, true;

    lmr_improving = 215, 100..=400, true;
    lmr_ttpv = 1200, 600..=1800, true;
    lmr_tt_score = 454, 250..=600, true;
    lmr_tt_depth = 303, 150..=500, true;
    lmr_history = 439, 300..=800, true;

    // Additional LMR terms
    lmr_corrplexity = 1536, 500..=2500, true;
    lmr_cutnode = 1200, 600..=1800, true;
    lmr_cutnode_no_tt = 2000, 1000..=3000, true;
    lmr_direct_check = 800, 400..=1400, true;

    // Quiet History
    hist_quiet_bonus_mult = 325, 150..=500,   true;
    hist_quiet_bonus_max  = 947, 800..=1400,  true;
    hist_quiet_bonus_base = 225, 150..=400,   true;
    hist_quiet_malus_mult = 289, 150..=500,   true;
    hist_quiet_malus_max  = 948, 800..=1400,  true;
    hist_quiet_malus_base = 235, 150..=400,   true;

    // Noisy History
    hist_noisy_bonus_mult = 253, 150..=500,   true;
    hist_noisy_bonus_max  = 1060, 800..=1400, true;
    hist_noisy_bonus_base = 190, 150..=400,   true;
    hist_noisy_malus_mult = 298, 150..=500,   true;
    hist_noisy_malus_max  = 938, 800..=1400,  true;
    hist_noisy_malus_base = 271, 150..=400,   true;

    // Continuation History
    hist_cont_bonus_mult = 315, 150..=500,    true;
    hist_cont_bonus_max  = 1056, 800..=1400,  true;
    hist_cont_bonus_base = 194, 150..=400,    true;
    hist_cont_malus_mult = 305, 150..=500,    true;
    hist_cont_malus_max  = 1082, 800..=1400,  true;
    hist_cont_malus_base = 270, 150..=400,    true;

    // Prior Counter Move
    pcm_scale = 122, 80..=300, true;
    pcm_offset = 76, 45..=120, true;
    pcm_max = 1194, 800..=1600, true;

    // Correction History
    corr_hist_base = 157, 50..=400,        true;
    corr_hist_min  = -4605, -6000..=-2000, true;
    corr_hist_max  = 2548, 1000..=5000,    true;

    // LMR correction history
    lmr_hist_base = 157, 50..=400, true;
    lmr_hist_min = -4605, -6000..=-2000, true;
    lmr_hist_max = 2548, 1000..=5000, true;

    // Qsearch
    qsearch_see        = -112, -400..=150, true;
    qsearch_hist_bonus =  106,   50..=300,  true;

    // SEE
    see_pawn   = 88,   50..=150,    true;
    see_knight = 429,  350..=450,   true;
    see_bishop = 454,  400..=550,   true;
    see_rook   = 654,  600..=800,   true;
    see_queen  = 1293, 1000..=1500, true;

    // Move Picker
    mp_see_base = 64, 40..=100, true;

    // Score Noisy
    score_queen_promo = 4885, 2500..=10000, true;

    // Score Quiet
    score_quiet_pawn   = 1006, 500..=1500,   true;
    score_quiet_cont1  = 1602, 1200..=2000,  true;
    score_quiet_cont2  = 1059, 800..=1600,   true;
    score_quiet_cont4  = 1066, 600..=1600,   true;
    direct_check_bonus = 9779, 9000..=12000, true;

    // Max Histories
    max_quiet_history    = 8062,  7500..=15000,  true;
    max_noisy_history    = 8292,  7500..=15000,  true;
    max_cont_history     = 7604,  7500..=15000,  true;
    max_contcorr_history = 11986, 7500..=15000,  true;
    max_corr_history     = 11983, 7500..=15000,  true;
    max_pawn_history     = 8038,  7500..=15000,  true;

    // Material Scaling
    mat_scale_pawn   = 50,    50..=150,       true;
    mat_scale_knight = 420,   350..=450,      true;
    mat_scale_bishop = 455,   400..=550,      true;
    mat_scale_rook   = 651,   600..=800,      true;
    mat_scale_queen  = 1216,  1000..=1500,    true;
    mat_scale_base   = 24792, 10000..=40000,  true;

    // Piece Values
    value_pawn   = 99,    50..=150,     true;
    value_knight = 319,   250..=325,     true;
    value_bishop = 331,   325..=450,     true;
    value_rook   = 504,   460..=800,     true;
    value_queen  = 899,   850..=1500,    true;
}
