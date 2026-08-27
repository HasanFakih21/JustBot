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
                    let c_end = (($max - $min) / 20).max(1);
                    let r_end = 0.002 / f32::min(0.5, c_end as f32) / 0.5;
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
    reset_delta = 25, 5..=50, true;

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
    lmr_quiet_base = 5551, 2550..=8550, true;
    lmr_noisy_base = 1051, -3050..=4050, true;

    lmr_quiet_div = 24482, 20000..=28500, true;
    lmr_noisy_div = 30004, 25000..=35000, true;

    lmr_improving = 217, 100..=400, true;
    lmr_ttpv = 197, 100..=400, true;
    lmr_tt_score = 447, 250..=600, true;
    lmr_tt_depth = 296, 150..=500, true;
    lmr_history = 449, 300..=800, true;
}
