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
    asp_init_alpha_window = 31, 15..=100, true;
    asp_init_beta_window  = 20, 15..=100, true;
    asp_alpha_window      = 31, 15..=100, true;
    asp_beta_window       = 20, 15..=100, true;

    // Base Time Scaling
    tm_base    = 60, 50..=75, true;
    tm_mult    = 50, 30..=65, true;
    tm_fm_mult = 35, 20..=50, true;

    // Node Time Scaling
    node_tm_base = 3000, 2500..=3500, true;
    node_tm_mult = 2500, 2000..=3000, true;
    node_tm_min  = 550,  450..=750,   true;

    // Razoring
    razoring_base = 250, 100..=400, true;
    razoring_mult = 250, 100..=400, true;

    // RFP
    rfp_base      = 148, 100..=300, true;
    rfp_improving = 92,  50..=150,  true;
    rfp_lerp_t    = 700, 300..=900, true;

    // NMP
    nmp_depth     = 3,   2..=4,     true;
    nmp_improving = 60,  30..=150,  true;
    nmp_base_r    = 6,   3..=9,     true;
    nmp_r_mult    = 128, 100..=300, true;
    nmp_r_div     = 640, 400..=900, true;

    // SE
    se_depth       = 6,   4..=8,     true;
    se_tt_depth    = 3,   1..=5,     true;
    se_double_base = 10,  5..=50,    true;
    se_double_pv   = 250, 100..=400, true;
    se_neg_ext     = 2,   1..=4,     true;

    // LMP
    lmp_base = 3000, 2000..=5000, true;
    lmp_mult = 1500, 1000..=3000, true;

    // FP
    fp_depth = 7, 4..=9,       true;
    fp_mult  = 90, 50..=150,   true;
    fp_base  = 146, 100..=200, true;

    // HP
    hp_depth = 5,    3..=8,       true;
    hp_base  = 1482, 1000..=2000, true;

    // Main SEE
    see_mult1 = 123, 100..=250, true;
    see_mult2 = 47,  20..=150,  true;
    see_base  = 13,  5..=50,    true;
    see_max   = 33,  5..=100,   true;

    // Base LMR
    lmr_base      = 7844, 500..=1200,   true;
    lmr_quiet_div = 24696, 2000..=3200, true;
    lmr_noisy_div = 30000, 2500..=3600, true;

    // LMR
    lmr_depth     = 2, 1..=3,       true;
    lmr_improving = 217, 150..=400, true;
    lmr_ttpv      = 200, 100..=500, true;
    lmr_tt_alpha  = 450, 100..=600, true;
    lmr_tt_depth  = 300, 150..=600, true;
    lmr_history   = 450, 150..=800, true;

    // Quiet History
    hist_quiet_bonus_mult = 319, 150..=500,   true;
    hist_quiet_bonus_max  = 928, 800..=1400,  true;
    hist_quiet_bonus_base = 227, 150..=400,   true;
    hist_quiet_malus_mult = 287, 150..=500,   true;
    hist_quiet_malus_max  = 955, 800..=1400,  true;
    hist_quiet_malus_base = 236, 150..=400,   true;

    // Noisy History
    hist_noisy_bonus_mult = 259, 150..=500,   true;
    hist_noisy_bonus_max  = 1060, 800..=1400, true;
    hist_noisy_bonus_base = 198, 150..=400,   true;
    hist_noisy_malus_mult = 308, 150..=500,   true;
    hist_noisy_malus_max  = 934, 800..=1400,  true;
    hist_noisy_malus_base = 277, 150..=400,   true;

    // Continuation History
    hist_cont_bonus_mult = 308, 150..=500,    true;
    hist_cont_bonus_max  = 1060, 800..=1400,  true;
    hist_cont_bonus_base = 196, 150..=400,    true;
    hist_cont_malus_mult = 303, 150..=500,    true;
    hist_cont_malus_max  = 1081, 800..=1400,  true;
    hist_cont_malus_base = 270, 150..=400,    true;

    // Correction History
    corr_hist_base = 150, 50..=400,        true;
    corr_hist_div  = 120, 80..=160,        true;
    corr_hist_min  = -4500, -6000..=-2000, true;
    corr_hist_max  = 2500, 1000..=5000,    true;
    corr_div       = 64, 50..=90,          true;

    // Qsearch
    qsearch_lmp        =  3,    2..=5,     true;
    qsearch_see        = -134, -400..=150, true;
    qsearch_hist_bonus =  98,   50..=300,  true;

    // SEE
    see_pawn   = 87,   50..=150,    true;
    see_knight = 429,  350..=450,   true;
    see_bishop = 456,  400..=550,   true;
    see_rook   = 661,  600..=800,   true;
    see_queen  = 1291, 1000..=1500, true;

    // Move Picker
    mp_see_base = 65, 40..=100, true; 
    mp_see_div  = 4,  3..=6,    true;

    // Score Noisy
    score_noisy_div = 8, 5..=10, true;

    // Score Quiet
    score_quiet_cont1 = 1588, 1200..=2000, true;
    score_quiet_cont2 = 1040, 800..=1600,  true;

    // Max Histories
    max_quiet_history = 8199, 7500..=15000,  true;
    max_noisy_history = 8113, 7500..=15000,  true;
    max_cont_history  = 7890, 7500..=15000,  true;
    max_corr_history  = 12000, 7500..=15000, true;
}
