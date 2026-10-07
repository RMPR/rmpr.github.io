//! Flux definitions: which base action maps to which Flux action per team,
//! and what each one costs.

use crate::data::FluxKind;

pub const POOL_MAX: f32 = 300.0;
pub const CHARGE: f32 = 100.0;
pub const STRAIN_FATIGUE: f32 = 70.0;
pub const STRAIN_BURNOUT: f32 = 90.0;
pub const DUEL_WINDOW: f32 = 0.4;
pub const DUEL_RANGE: f32 = 3.5;
pub const TEAM_FLUX_DURATION: f32 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BaseAction {
    Sprint,
    Shoot,
    Pass,
    Lob,
    Tackle,
    Special,
    Keeper,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum FluxAction {
    // Breath of Akillian
    #[default]
    Burst,
    AkillianStrike,
    IceLane,
    HighBreath,
    FrostLock,
    SuperJump,
    WallOfIce,
    // Smog
    SmogStep,
    SmogShot,
    VeiledPass,
    PhaseLob,
    SmogSnatch,
    EclipseCloud,
    ShadowKeeper,
}

#[derive(Clone, Copy, Debug)]
pub struct FluxSpec {
    pub cost: f32,
    pub strain: f32,
    pub wind_up: f32,
    pub duration: f32,
}

impl FluxAction {
    pub fn kind(self) -> FluxKind {
        use FluxAction::*;
        match self {
            Burst | AkillianStrike | IceLane | HighBreath | FrostLock | SuperJump | WallOfIce => {
                FluxKind::Breath
            }
            _ => FluxKind::Smog,
        }
    }

    pub fn spec(self) -> FluxSpec {
        use FluxAction::*;
        let s = |cost, strain, wind_up, duration| FluxSpec { cost, strain, wind_up, duration };
        match self {
            Burst => s(CHARGE, 20.0, 0.0, 1.5),
            AkillianStrike => s(CHARGE, 25.0, 0.25, 0.1),
            IceLane => s(CHARGE, 15.0, 0.1, 0.1),
            HighBreath => s(CHARGE, 15.0, 0.1, 0.1),
            FrostLock => s(CHARGE, 15.0, 0.0, 0.5),
            SuperJump => s(CHARGE, 20.0, 0.0, 0.6),
            WallOfIce => s(CHARGE, 15.0, 0.0, 1.0),
            SmogStep => s(CHARGE, 20.0, 0.3, 0.1),
            SmogShot => s(CHARGE, 25.0, 0.2, 0.1),
            VeiledPass => s(CHARGE, 15.0, 0.1, 0.1),
            PhaseLob => s(CHARGE, 15.0, 0.1, 0.1),
            SmogSnatch => s(CHARGE, 20.0, 0.2, 0.5),
            EclipseCloud => s(CHARGE, 15.0, 0.0, 3.0),
            ShadowKeeper => s(CHARGE, 15.0, 0.0, 0.5),
        }
    }

    pub fn name(self) -> &'static str {
        use FluxAction::*;
        match self {
            Burst => "Breath Burst",
            AkillianStrike => "Akillian Strike",
            IceLane => "Ice Lane",
            HighBreath => "High Breath",
            FrostLock => "Frost Lock",
            SuperJump => "Super Jump",
            WallOfIce => "Wall of Ice",
            SmogStep => "Smog Step",
            SmogShot => "Smog Shot",
            VeiledPass => "Veiled Pass",
            PhaseLob => "Phase Lob",
            SmogSnatch => "Smog Snatch",
            EclipseCloud => "Eclipse Cloud",
            ShadowKeeper => "Shadow Keeper",
        }
    }
}

pub fn map(kind: FluxKind, base: BaseAction) -> FluxAction {
    use BaseAction::*;
    use FluxAction::*;
    match (kind, base) {
        (FluxKind::Breath, Sprint) => Burst,
        (FluxKind::Breath, Shoot) => AkillianStrike,
        (FluxKind::Breath, Pass) => IceLane,
        (FluxKind::Breath, Lob) => HighBreath,
        (FluxKind::Breath, Tackle) => FrostLock,
        (FluxKind::Breath, Special) => SuperJump,
        (FluxKind::Breath, Keeper) => WallOfIce,
        (FluxKind::Smog, Sprint) => SmogStep,
        (FluxKind::Smog, Shoot) => SmogShot,
        (FluxKind::Smog, Pass) => VeiledPass,
        (FluxKind::Smog, Lob) => PhaseLob,
        (FluxKind::Smog, Tackle) => SmogSnatch,
        (FluxKind::Smog, Special) => EclipseCloud,
        (FluxKind::Smog, Keeper) => ShadowKeeper,
    }
}

/// How the pool charges. Breath charges on momentum, Smog on disruption.
#[derive(Clone, Copy, Debug)]
pub enum PoolGain {
    PassComplete,
    TackleWon,
    Interception,
    ShotOnTarget,
    SprintWithBallSecond,
    PressSecond,
    Conceded,
    DribblePast,
}

pub fn pool_gain(kind: FluxKind, g: PoolGain) -> f32 {
    use PoolGain::*;
    match (kind, g) {
        (FluxKind::Breath, PassComplete) => 12.0,
        (FluxKind::Breath, TackleWon) => 6.0,
        (FluxKind::Breath, Interception) => 6.0,
        (FluxKind::Breath, ShotOnTarget) => 15.0,
        (FluxKind::Breath, SprintWithBallSecond) => 4.0,
        (FluxKind::Breath, PressSecond) => 0.0,
        (FluxKind::Breath, Conceded) => 40.0,
        (FluxKind::Breath, DribblePast) => 10.0,
        (FluxKind::Smog, PassComplete) => 6.0,
        (FluxKind::Smog, TackleWon) => 15.0,
        (FluxKind::Smog, Interception) => 15.0,
        (FluxKind::Smog, ShotOnTarget) => 8.0,
        (FluxKind::Smog, SprintWithBallSecond) => 0.0,
        (FluxKind::Smog, PressSecond) => 5.0,
        (FluxKind::Smog, Conceded) => 50.0,
        (FluxKind::Smog, DribblePast) => 4.0,
    }
}
