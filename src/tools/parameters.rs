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

tunable_params! {
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
