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
                    stringify!($name) => vals::$name.store(val.clamp($min, $max), Ordering::Relaxed),
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
        const _: () = assert!($min <= $val && $val <= $max && $min < $max);

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
    init_delta = 24, 5..=50, true;
    fail_low_delta = 23, 5..=50, true;
    fail_high_delta = 28, 5..=50, true;
    asp_fail_high_reduction = 3, 0..=5, true;

    // Time Management (values are divided by 1000)
    tm_base = 60, 50..=75, true;
    tm_mult = 50, 30..=65, true;
    tm_fm_mult = 35, 20..=50, true;
    tm_hard_scale = 750, 500..=1000, true;
    tm_inc_scale = 750, 500..=1000, true;
    tm_cyclic_soft_scale = 1000, 500..=2000, false;
    tm_cyclic_hard_scale = 5000, 2000..=8000, false;

    // Node Time Scaling
    node_tm_base = 2977, 2500..=3500, true;
    node_tm_mult = 2495, 2000..=3000, true;
    node_tm_min = 553, 450..=750, true;

    // Score Trend Time Scaling
    score_tm_base = 750, 500..=1000, true;
    score_tm_mult = 45, 0..=100, true;
    score_tm_min = 700, 400..=900, true;
    score_tm_max = 1500, 1000..=2000, true;

    // Move Stability Time Scaling
    move_tm_base = 1200, 1000..=1500, true;
    move_tm_mult = 50, 10..=150, true;
    move_tm_min = 850, 500..=1000, true;

    // TT Cutoffs
    tt_cutoff_quiet_scale = 188, 50..=500, true;
    tt_cutoff_quiet_offset = 54, 5..=150, true;
    tt_cutoff_quiet_max = 1167, 800..=1800, true;
    tt_cutoff_halfmove = 90, 70..=99, true;

    // TT Replacement
    tt_age_penalty = 4, 1..=8, true;
    tt_replace_depth_margin = 4, 0..=8, true;
    tt_replace_pv_bonus = 2, 0..=4, true;

    // Evaluation Difference History
    eval_hist_depth = 6, 2..=12, true;
    eval_hist_scale = 10, 1..=30, true;
    eval_hist_min = -65, -300..=0, true;
    eval_hist_max = 200, 1..=500, true;

    // Hindsight
    hindsight_ext_reduction = 3167, 2100..=4600, true;
    hindsight_ext_eval = 0, -200..=200, true;
    hindsight_red_depth = 2, 2..=6, true;
    hindsight_red_reduction = 2081, 1024..=3048, true;
    hindsight_red_eval = 214, 5..=500, true;

    // Razoring
    razoring_offset = 237, 100..=400, true;
    razoring_scale = 262, 100..=400, true;
    razoring_max_alpha = 2000, 500..=4000, true;

    // RFP
    rfp_scale_1 = 93, 40..=140, true;
    rfp_scale_2 = 7, 1..=20, true;
    rfp_improving = 72, 40..=150, true;
    rfp_t = 666, 500..=900, true;

    // NMP
    nmp_depth = 3, 2..=6, true;
    nmp_offset = 217, 100..=400, true;
    nmp_scale = 1274, 800..=1500, true;
    nmp_improving = 65, 35..=100, true;
    nmp_r_scale = 134, 40..=300, true;
    nmp_base_r = 6, 3..=9, true;
    nmp_r_div = 640, 400..=900, true;
    nmp_verify_depth = 14, 8..=20, true;
    nmp_verify_scale = 768, 256..=1024, true;

    // ProbCut
    probcut_depth = 3, 2..=6, true;
    probcut_reduction = 3, 1..=6, true;
    probcut_margin = 262, 100..=400, true;
    probcut_improving = 50, 0..=100, true;

    // Internal Iterative Reduction
    iir_depth = 4, 2..=8, true;
    iir_reduction = 1, 0..=2, true;

    // Singular Extensions
    se_depth = 5, 4..=8, true;
    se_tt_depth = 3, 1..=5, true;
    se_depth_offset = 1, 0..=3, true;
    se_depth_div = 2, 2..=4, true;
    se_margin_scale = 1024, 512..=2048, true;
    se_margin_ttpv = 1024, 0..=2048, true;
    se_double_base = 9, 5..=30, true;
    se_double_pv = 142, 50..=300, true;
    se_double_new_pv = 55, 10..=100, true;
    se_triple_base = 104, 50..=200, true;
    se_triple_pv = 358, 250..=450, true;
    se_triple_new_pv = 66, 10..=200, true;
    se_neg_ext = 3, 1..=4, true;

    // LMP
    lmp_base1 = 2896, 2000..=4500, true;
    lmp_base2 = 1286, 500..=2500, true;
    lmp_improving = 266, 150..=450, true;

    // FP
    fp_depth = 14, 8..=20, true;
    fp_scale = 76, 40..=200, true;
    fp_offset = 162, 50..=350, true;
    fp_history = 57, 15..=150, true;

    // HP
    hp_depth = 6, 3..=10, true;
    hp_scale = -1464, -2000..=-500, true;

    // Main Search SEE Pruning
    main_see_quiet_scale = -20, -100..=-1, true;
    main_see_quiet_scale2 = 50, 0..=150, true;
    main_see_quiet_offset = 25, 0..=100, true;
    main_see_quiet_history = 25, 0..=100, true;
    main_see_quiet_max = 0, -100..=0, true;

    main_see_scale = -10, -100..=-1, true;
    main_see_scale2 = 34, 25..=150, true;
    main_see_offset = 13, 5..=50, true;
    main_see_history = 35, 0..=100, true;
    main_see_max = 0, -100..=0, true;

    // LMR
    lmr_depth = 2, 2..=5, true;
    lmr_move_count = 1, 1..=5, true;
    lmr_quiet_base = 6048, 2550..=8550, true;
    lmr_noisy_base = 1546, -3050..=4050, true;

    lmr_quiet_div = 24978, 20000..=28500, true;
    lmr_noisy_div = 30169, 25000..=35000, true;

    lmr_improving = 202, 100..=400, true;
    lmr_ttpv = 1240, 600..=1800, true;
    lmr_ttpv_score = 600, 0..=1200, true;
    lmr_ttpv_depth = 600, 0..=1200, true;
    lmr_tt_score = 474, 250..=600, true;
    lmr_tt_depth = 294, 150..=500, true;
    lmr_history = 440, 300..=800, true;

    // Additional LMR terms
    lmr_corrplexity = 1653, 500..=2500, true;
    lmr_cutnode = 1270, 600..=1800, true;
    lmr_cutnode_no_tt = 1984, 1000..=3000, true;
    lmr_direct_check = 859, 400..=1400, true;
    lmr_offset = 333, 0..=1024, true;
    lmr_fail_highs = 500, 0..=1500, true;
    lmr_fail_high_count = 2, 0..=5, true;
    lmr_deeper_margin = 50, 10..=150, true;
    lmr_shallower_margin = 8, 0..=30, true;

    // History Weights for Pruning and Reductions (divided by 1024)
    search_history_quiet = 1024, 512..=1536, true;
    search_history_cont1 = 1024, 512..=1536, true;
    search_history_cont2 = 1024, 512..=1536, true;
    search_history_noisy = 1024, 512..=1536, true;

    // Cutoff Score Softening
    cutoff_soften_weight = 1, 1..=4, true;

    // Quiet History
    hist_quiet_bonus_mult = 294, 150..=500,   true;
    hist_quiet_bonus_max  = 944, 800..=1400,  true;
    hist_quiet_bonus_base = 232, 150..=400,   true;
    hist_quiet_malus_mult = 296, 150..=500,   true;
    hist_quiet_malus_max  = 951, 800..=1400,  true;
    hist_quiet_malus_base = 211, 150..=400,   true;

    // Noisy History
    hist_noisy_bonus_mult = 244, 150..=500,   true;
    hist_noisy_bonus_max  = 1054, 800..=1400, true;
    hist_noisy_bonus_base = 203, 150..=400,   true;
    hist_noisy_malus_mult = 288, 150..=500,   true;
    hist_noisy_malus_max  = 962, 800..=1400,  true;
    hist_noisy_malus_base = 284, 150..=400,   true;

    // Continuation History
    hist_cont_bonus_mult = 293, 150..=500,    true;
    hist_cont_bonus_max  = 1019, 800..=1400,  true;
    hist_cont_bonus_base = 196, 150..=400,    true;
    hist_cont_malus_mult = 321, 150..=500,    true;
    hist_cont_malus_max  = 1105, 800..=1400,  true;
    hist_cont_malus_base = 266, 150..=400,    true;

    // Prior Counter Move
    pcm_scale = 115, 80..=300, true;
    pcm_offset = 75, 45..=120, true;
    pcm_max = 1203, 800..=1600, true;

    // Correction History
    corr_hist_base = 201, 50..=400,        true;
    corr_hist_div  = 128, 80..=180,       true;
    corr_hist_min  = -4419, -6000..=-2000, true;
    corr_hist_max  = 2508, 1000..=5000,    true;
    corr_div = 64, 50..=90, true;

    // Qsearch
    qsearch_lmp        = 3, 2..=6, true;
    qsearch_see        = -82, -400..=150, true;
    qsearch_hist_bonus =  119,   50..=300,  true;
    qsearch_standpat_t = 800, 500..=1024, true;

    // SEE
    see_pawn   = 89,   50..=150,    true;
    see_knight = 435,  350..=450,   true;
    see_bishop = 460,  400..=550,   true;
    see_rook   = 665,  600..=800,   true;
    see_queen  = 1237, 1000..=1500, true;

    // Move Picker
    mp_see_base = 59, 40..=100, true;
    mp_see_div = 4, 2..=8, true;

    // Score Noisy
    score_queen_promo = 5129, 2500..=10000, true;
    score_noisy_div = 8, 4..=16, true;

    // Score Quiet
    score_quiet        = 1024, 500..=1500,   true;
    score_quiet_pawn   = 1081, 500..=1500,   true;
    score_quiet_cont1  = 1573, 1200..=2000,  true;
    score_quiet_cont2  = 1064, 800..=1600,   true;
    score_quiet_cont4  = 1048, 600..=1600,   true;
    direct_check_bonus = 9690, 9000..=12000, true;

    // Max Histories
    max_quiet_history    = 8083,  7500..=15000,  true;
    max_noisy_history    = 8649,  7500..=15000,  true;
    max_cont_history     = 7851,  7500..=15000,  true;
    max_contcorr_history = 11805, 7500..=15000,  true;
    max_corr_history     = 11921, 7500..=15000,  true;
    max_pawn_history     = 7944,  7500..=15000,  true;

    // Material Scaling
    mat_scale_pawn   = 53,    50..=150,       true;
    mat_scale_knight = 418,   350..=450,      true;
    mat_scale_bishop = 451,   400..=550,      true;
    mat_scale_rook   = 660,   600..=800,      true;
    mat_scale_queen  = 1234,  1000..=1500,    true;
    mat_scale_base   = 24301, 10000..=40000,  true;
    eval_halfmove_scale = 200, 101..=300, true;

    // Piece Values
    value_pawn   = 105,    50..=150,     true;
    value_knight = 318,   250..=325,     true;
    value_bishop = 328,   325..=450,     true;
    value_rook   = 503,   460..=800,     true;
    value_queen  = 903,   850..=1500,    true;
}
