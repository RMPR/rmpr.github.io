use crate::pitch::*;
use glam::{Vec2, Vec3};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BallFlags {
    /// Breath: the pass slides on an ice lane, only Flux tackles intercept it.
    pub ice_lane: bool,
    /// Smog: the ball is hidden until this sim time.
    pub hidden_until: f32,
    /// Smog: the next interception by an opponent is ignored.
    pub phase: bool,
    /// Flux shot: harder to save, parries knock the keeper back.
    pub flux_shot: bool,
    /// Breath: receiver may super-jump to meet this lob.
    pub high_breath: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ball {
    pub p: Vec3,
    pub v: Vec3,
    /// Signed curl around the vertical axis.
    pub spin: f32,
    pub owner: Option<usize>,
    pub last_touch: Option<usize>,
    pub last_team: Option<usize>,
    /// Seconds since the last kick; a player cannot re-collect their own kick immediately.
    pub since_kick: f32,
    pub kicked_by: Option<usize>,
    pub flags: BallFlags,
    /// Previous tick position, for line-crossing tests.
    pub prev: Vec3,
}

impl Default for Ball {
    fn default() -> Self {
        Ball {
            p: Vec3::new(0.0, 0.0, BALL_RADIUS),
            v: Vec3::ZERO,
            spin: 0.0,
            owner: None,
            last_touch: None,
            last_team: None,
            since_kick: 10.0,
            kicked_by: None,
            flags: BallFlags::default(),
            prev: Vec3::new(0.0, 0.0, BALL_RADIUS),
        }
    }
}

const GRAVITY: f32 = 9.81;
const AIR_DRAG: f32 = 0.012;
const GROUND_FRICTION: f32 = 0.55;
const ROLL_DECEL: f32 = 2.0;
const BOUNCE: f32 = 0.6;
const MAGNUS: f32 = 0.08;

impl Ball {
    pub fn xy(&self) -> Vec2 {
        Vec2::new(self.p.x, self.p.y)
    }

    pub fn place(&mut self, at: Vec2) {
        self.p = Vec3::new(at.x, at.y, BALL_RADIUS);
        self.prev = self.p;
        self.v = Vec3::ZERO;
        self.spin = 0.0;
        self.owner = None;
        self.kicked_by = None;
        self.since_kick = 10.0;
        self.flags = BallFlags::default();
    }

    /// Free-flight integration. Returns the ground-bounce speed if it bounced.
    pub fn integrate(&mut self, dt: f32, friction_mult: f32) -> Option<f32> {
        self.prev = self.p;
        self.since_kick += dt;
        let speed = self.v.length();
        if speed > 0.0 {
            let drag = AIR_DRAG * speed;
            let mut acc = -self.v * drag;
            // Magnus: spin curls the horizontal velocity sideways.
            let side = Vec3::new(-self.v.y, self.v.x, 0.0) * (self.spin * MAGNUS);
            acc += side;
            acc.z -= GRAVITY;
            self.v += acc * dt;
        } else {
            self.v.z -= GRAVITY * dt;
        }
        self.p += self.v * dt;
        let mut bounced = None;
        if self.p.z <= BALL_RADIUS {
            self.p.z = BALL_RADIUS;
            if self.v.z < -0.5 {
                bounced = Some(-self.v.z);
                self.v.z = -self.v.z * BOUNCE;
                self.spin *= 0.7;
            } else {
                self.v.z = 0.0;
            }
            // Rolling friction.
            let h = Vec2::new(self.v.x, self.v.y);
            let hs = h.length();
            if hs > 0.0 {
                let f = friction_mult * (GROUND_FRICTION * hs + ROLL_DECEL) * dt;
                let nh = if f >= hs { Vec2::ZERO } else { h - h / hs * f };
                self.v.x = nh.x;
                self.v.y = nh.y;
            }
            self.spin *= 1.0 - 1.5 * dt;
        }
        // Goal frame: posts and crossbar as a crude box test so shots hitting
        // the frame rebound instead of vanishing.
        for dir in [-1.0f32, 1.0] {
            let gx = goal_x(dir);
            let crossing = (self.prev.x - gx) * dir < 0.0 && (self.p.x - gx) * dir >= 0.0
                || (self.prev.x - gx) * dir >= 0.0 && (self.p.x - gx) * dir < 0.0;
            if crossing {
                let t = ((gx - self.prev.x) / (self.p.x - self.prev.x)).clamp(0.0, 1.0);
                let hit = self.prev.lerp(self.p, t);
                let near_post = (hit.y.abs() - GOAL_HALF_WIDTH).abs() < BALL_RADIUS + 0.06
                    && hit.z < GOAL_HEIGHT + 0.1;
                let bar = hit.y.abs() <= GOAL_HALF_WIDTH + 0.06
                    && (hit.z - GOAL_HEIGHT).abs() < BALL_RADIUS + 0.06;
                if near_post || bar {
                    self.p = hit - Vec3::new(dir * 0.05, 0.0, 0.0);
                    self.v.x = -self.v.x * 0.7;
                    self.v.y *= 0.8;
                    bounced = Some(self.v.length());
                }
            }
        }
        bounced
    }
}
