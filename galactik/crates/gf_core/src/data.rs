//! Static definitions: teams, players, stats. Loaded from RON files that are
//! embedded in the binary (`builtin_teams`), or parsed at runtime.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pos {
    GK,
    DF,
    MF,
    ST,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FluxKind {
    /// The Breath of Akillian: explosive, vertical, clean.
    Breath,
    /// The Smog: teleportation, concealment, sickness for non-natives.
    Smog,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    pub pace: u8,
    pub acceleration: u8,
    pub ball_control: u8,
    pub dribbling: u8,
    pub short_pass: u8,
    pub long_pass: u8,
    pub shooting: u8,
    pub heading: u8,
    pub tackle: u8,
    pub strength: u8,
    pub stamina: u8,
    pub reactions: u8,
    pub gk_reflex: u8,
    pub gk_reach: u8,
    pub flux_power: u8,
    pub flux_control: u8,
}

impl Stats {
    /// Normalised stat in 0..=1.
    #[inline]
    pub fn n(v: u8) -> f32 {
        v as f32 / 99.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerDef {
    pub id: String,
    pub name: String,
    pub number: u8,
    pub pos: Pos,
    #[serde(default = "default_true")]
    pub native: bool,
    #[serde(default)]
    pub starter: bool,
    pub stats: Stats,
    #[serde(default)]
    pub signature: Option<String>,
    /// Hex colour of the hair/head accent, used by the placeholder look.
    #[serde(default)]
    pub accent: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Kit {
    pub primary: String,
    pub secondary: String,
    pub accent: String,
}

/// Seven anchors in team-relative coordinates: `x` in 0..1 from own goal
/// line to the opponent's, `y` in -1..1 across the width. Order matches the
/// starters list (goalkeeper first).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Formation {
    pub name: String,
    pub anchors: Vec<(f32, f32)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TeamDef {
    pub id: String,
    pub name: String,
    pub short_name: String,
    pub home: String,
    pub coach: String,
    pub flux: FluxKind,
    pub kit: Kit,
    pub attack: Formation,
    pub defend: Formation,
    pub captain: String,
    pub team_flux: String,
    pub players: Vec<PlayerDef>,
}

impl TeamDef {
    pub fn parse(ron_src: &str) -> Result<TeamDef, ron::error::SpannedError> {
        ron::from_str(ron_src)
    }

    pub fn starters(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.starter)
            .map(|(i, _)| i)
            .collect();
        // Goalkeeper first, then by shirt number, so anchors line up.
        v.sort_by_key(|&i| (self.players[i].pos != Pos::GK, self.players[i].number));
        v.truncate(7);
        v
    }
}

pub const SNOW_KIDS_RON: &str = include_str!("../../../assets/teams/snow_kids.ron");
pub const SHADOWS_RON: &str = include_str!("../../../assets/teams/shadows.ron");

/// The two launch teams, parsed from the embedded RON files.
pub fn builtin_teams() -> Vec<TeamDef> {
    vec![
        TeamDef::parse(SNOW_KIDS_RON).expect("snow_kids.ron"),
        TeamDef::parse(SHADOWS_RON).expect("shadows.ron"),
    ]
}
