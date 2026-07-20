use crate::tunable_params;

#[rustfmt::skip]
tunable_params! {
    asp_alpha_window = 25, 5..=300, true;
    asp_beta_window  = 25, 5..=300, true;
    asp_multiplier   = 2, 1..=4,    true;

    rfp_base      = 150, 30..=500,   true;
    rfp_improving = 100, 20..=500,   true;

    nmp_improving = 50, 5..=300,     true;
    nmp_depth     = 4, 1..=10,       true;

    lmp_base      = 3, 1..=6,        true;
    lmp_improving = 2, 1..=4,        true;

    fp_depth      = 6, 3..=9,        true;
    fp_base       = 100, -20..=400,  true;
    fp_offset     = 150, -5..=300,   true;

    see_base      = -10, -200..=300, true;
    see_offset1   = 30, -200..=300,  true;
    see_offset2   = 15, -100..=150,  true;
    see_min       = 0, -400..= 400,  true; 

    lmr_depth     = 3, 1..=7,        true;
    lmr_improving = 200, -200..=700, true;
    lmr_pv        = 2, 1..=5,        true;

    quiet_hist_bonus_base   = 300,  40..=1000,   true;
    quiet_hist_bonus_min    = 1000, 400..=3000,  true;
    quiet_hist_bonus_offset = 250, -200..= 900,  true;
    quiet_hist_malus_base   = 300,  40..=1000,   true;
    quiet_hist_malus_min    = 1000, 400..=3000,  true;
    quiet_hist_malus_offset = 250, -200..= 900,  true;

    noisy_hist_bonus_base   = 250,  40..=1000,   true;
    noisy_hist_bonus_min    = 1000, 400..=3000,  true;
    noisy_hist_bonus_offset = 250, -200..= 900,  true;
    noisy_hist_malus_base   = 300,  40..=1000,   true;
    noisy_hist_malus_min    = 1000, 400..=3000,  true;
    noisy_hist_malus_offset = 250, -200..= 900,  true;

    cont_hist_bonus_base   = 350,  40..=1000,   true;
    cont_hist_bonus_min    = 1000, 400..=3000,  true;
    cont_hist_bonus_offset = 250, -200..= 900,  true;
    cont_hist_malus_base   = 250,  40..=1000,   true;
    cont_hist_malus_min    = 1000, 400..=3000,  true;
    cont_hist_malus_offset = 250, -200..= 900,  true;

    qsearch_lmp_move_count   = 4, 1..= 8,        true;
    qsearch_see              = -150, -600..=600, true;
    qsearch_noisy_hist_bonus = 100, -300..=400,  true; 

    good_noisy_see_divisor = 4, 2..=6,      true;
    good_noisy_see_offset  = 75, -40..=300, true;

    queen_promotion_bonus = 5000, 500..= 10000, true;

    noisy_hist_score_div  = 8, 1..=12, true;

    first_ply_conthistory_score_base  = 1600, 400..=4000, true;
    second_ply_conthistory_score_base = 1000, 400..=4000, true;
    direct_check_bonus                = 10000, 0..=10000, true;

    see_pawn_value    = 100, 20..=300,   true;
    see_knight_value  = 320, 200..=600,  true;
    see_bishop_value  = 330, 200..=700,  true;
    see_rook_value    = 500, 400..=900,  true;
    see_queen_value   = 900, 500..=1200, true;
}
