// Code is originally from akimbo: https://github.com/jw1912/akimbo under the MIT license
// Modifications from Hobbes: https://github.com/kelseyde/hobbes-chess-engine

#[cfg(feature = "tuning")]
#[macro_export]
macro_rules! tunable_params {
    ($($name:ident = $val:expr, $min:literal ..= $max:literal, $spsa:expr;)*) => {
        #[allow(unused)]
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

        #[allow(unused)]
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
                    let c_end = (($max - $min) as f32 / 20.0);
                    let r_end = 0.002 / c_end.min(0.5) / 0.5;
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

        mod vals {
            #[allow(unused)]
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

#[cfg(feature = "tuning")]
tunable_params! {
    // Aspiration Window
    init_delta = 25, 5..=50, true;
    fail_low_delta = 25, 5..=50, true;
    fail_high_delta = 25, 5..=50, true;

    // TT Cutoffs
    tt_cutoff_quiet_scale = 200, 50..=500, true;
    tt_cutoff_quiet_offset = 50, 5..=150, true;
    tt_cutoff_quiet_max = 1200, 800..=1800, true;

    // Hindsight
    hindight_ext_reduction = 3072, 2100..=4600, true;
    hindight_red_reduction = 2048, 1024..=3048, true;
    hindsight_red_eval = 200, 5..=500, true;

    // Razoring
    razoring_offset = 246, 100..=400, true;
    razoring_scale = 253, 100..=400, true;

    // RFP
    rfp_scale_1 = 85, 40..=140, true;
    rfp_scale_2 = 5, 1..=20, true;
    rfp_improving = 75, 40..=150, true;
    rfp_t = 690, 500..=900, true;

    // NMP
    nmp_offset = 200, 100..=400, true;
    nmp_scale = 1250, 800..=1500, true;
    nmp_improving = 63, 35..=100, true;
    nmp_r_scale = 132, 40..=300, true;

    // Singular Extensions
    se_double_base = 10, 5..=30, true;
    se_double_pv = 150, 50..=300, true;
    se_double_new_pv = 50, 10..=100, true;
    se_triple_base = 100, 50..=200, true;
    se_triple_pv = 350, 250..=450, true;
    se_triple_new_pv = 50, 10..=200, true;

    // LMP
    lmp_base1 = 3011, 2000..=4500, true;
    lmp_base2 = 1365, 500..=2500, true;
    lmp_improivng = 256, 150..=450, true;

    // FP
    fp_scale = 93, 40..=200, true;
    fp_offset = 146, 50..=350, true;
    fp_history = 50, 15..=150, true;

    // HP
    hp_scale = -1485, -2000..=-500, true;

    // Main Search SEE Pruning
    main_see_scale = -125, -400..=-20, true;
    main_see_scale2 = 46, 25..=150, true;
    main_see_offset = 14, 5..=50, true;
    main_see_max = -34, -100..=0, true;

    // LMR
    lmr_quiet_base = 5595, 2550..=8550, true;
    lmr_noisy_base = 1079, -3050..=4050, true;

    lmr_quiet_div = 24581, 20000..=28500, true;
    lmr_noisy_div = 30121, 25000..=35000, true;

    lmr_improving = 218, 100..=400, true;
    lmr_ttpv = 192, 100..=400, true;
    lmr_tt_score = 446, 250..=600, true;
    lmr_tt_depth = 292, 150..=500, true;
    lmr_history = 436, 300..=800, true;

    // Quiet History
    hist_quiet_bonus_mult = 321, 150..=500,   true;
    hist_quiet_bonus_max  = 935, 800..=1400,  true;
    hist_quiet_bonus_base = 228, 150..=400,   true;
    hist_quiet_malus_mult = 289, 150..=500,   true;
    hist_quiet_malus_max  = 948, 800..=1400,  true;
    hist_quiet_malus_base = 232, 150..=400,   true;

    // Noisy History
    hist_noisy_bonus_mult = 257, 150..=500,   true;
    hist_noisy_bonus_max  = 1058, 800..=1400, true;
    hist_noisy_bonus_base = 196, 150..=400,   true;
    hist_noisy_malus_mult = 302, 150..=500,   true;
    hist_noisy_malus_max  = 937, 800..=1400,  true;
    hist_noisy_malus_base = 273, 150..=400,   true;

    // Continuation History
    hist_cont_bonus_mult = 315, 150..=500,    true;
    hist_cont_bonus_max  = 1044, 800..=1400,  true;
    hist_cont_bonus_base = 194, 150..=400,    true;
    hist_cont_malus_mult = 303, 150..=500,    true;
    hist_cont_malus_max  = 1079, 800..=1400,  true;
    hist_cont_malus_base = 271, 150..=400,    true;

    // Prior Counter Move
    pcm_scale = 120, 80..=300, true;
    pcm_offset = 75, 45..=120, true;
    pcm_max = 1200, 800..=1600, true;

    // Correction History
    corr_hist_base = 148, 50..=400,        true;
    corr_hist_min  = -4612, -6000..=-2000, true;
    corr_hist_max  = 2530, 1000..=5000,    true;

    // Qsearch
    qsearch_see        = -129, -400..=150, true;
    qsearch_hist_bonus =  103,   50..=300,  true;

    // SEE
    see_pawn   = 88,   50..=150,    true;
    see_knight = 428,  350..=450,   true;
    see_bishop = 458,  400..=550,   true;
    see_rook   = 659,  600..=800,   true;
    see_queen  = 1289, 1000..=1500, true;

    // Move Picker
    mp_see_base = 64, 40..=100, true;

    // Score Noisy
    score_queen_promo = 5000, 2500..=10000, true;

    // Score Quiet
    score_quiet_pawn   = 1000, 500..=1500,   true;
    score_quiet_cont1  = 1595, 1200..=2000,  true;
    score_quiet_cont2  = 1050, 800..=1600,   true;
    score_quiet_cont4  = 1050, 600..=1600,   true;
    direct_check_bonus = 9808, 9000..=12000, true;

    // Max Histories
    max_quiet_history    = 8128,  7500..=15000,  true;
    max_noisy_history    = 8209,  7500..=15000,  true;
    max_cont_history     = 7813,  7500..=15000,  true;
    max_contcorr_history = 12000, 7500..=15000,  true;
    max_corr_history     = 11972, 7500..=15000,  true;
    max_pawn_history     = 8000,  7500..=15000,  true;

    // Material Scaling
    mat_scale_pawn   = 50,    50..=150,       true;
    mat_scale_knight = 420,   350..=450,      true;
    mat_scale_bishop = 458,   400..=550,      true;
    mat_scale_rook   = 650,   600..=800,      true;
    mat_scale_queen  = 1200,  1000..=1500,    true;
    mat_scale_base   = 26000, 10000..=40000, true;

    // Piece Values
    value_pawn   = 100,    50..=150,     true;
    value_knight = 320,   250..=325,     true;
    value_bishop = 330,   325..=450,     true;
    value_rook   = 500,   460..=800,     true;
    value_queen  = 900,   850..=1500,    true;
}
