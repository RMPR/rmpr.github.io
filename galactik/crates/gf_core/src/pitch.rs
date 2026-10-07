//! Pitch geometry. Sim coordinates: `x` along the length (team 0 attacks +x
//! in the first half), `y` across the width, `z` is height (ball only).

pub const HALF_LEN: f32 = 30.0;
pub const HALF_WID: f32 = 20.0;
pub const GOAL_HALF_WIDTH: f32 = 2.5;
pub const GOAL_HEIGHT: f32 = 2.0;
pub const GOAL_DEPTH: f32 = 1.5;
pub const BOX_DEPTH: f32 = 10.0;
pub const BOX_HALF_WIDTH: f32 = 9.0;
pub const PENALTY_SPOT: f32 = 7.0;
pub const CENTRE_CIRCLE_R: f32 = 5.0;
pub const BALL_RADIUS: f32 = 0.11;
pub const PLAYER_RADIUS: f32 = 0.4;

#[inline]
pub fn goal_x(dir: f32) -> f32 {
    HALF_LEN * dir
}

/// True if the point is inside the penalty box of the goal at `x = HALF_LEN*dir`.
pub fn in_box(p: glam::Vec2, dir: f32) -> bool {
    let depth = HALF_LEN - p.x * dir;
    (0.0..=BOX_DEPTH).contains(&depth) && p.y.abs() <= BOX_HALF_WIDTH
}
