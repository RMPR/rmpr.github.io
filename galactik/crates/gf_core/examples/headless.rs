//! Runs an AI vs AI match headlessly and prints statistics. Useful for tuning.
//! `cargo run -p gf_core --example headless --release -- [seed] [difficulty]`

use gf_core::*;
use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let difficulty: f32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.7);
    let teams = builtin_teams();
    let cfg = MatchConfig { seed, half_len_secs: 180.0, difficulty, human: [false, false] };
    let mut sim = Sim::new(teams[0].clone(), teams[1].clone(), cfg);
    let mut counts: BTreeMap<String, [u32; 2]> = BTreeMap::new();
    let mut possession = [0u32; 2];
    let start = std::time::Instant::now();
    while !matches!(sim.phase, Phase::FullTime) {
        sim.step([None, None]);
        if let Some(o) = sim.ball.owner {
            possession[sim.players[o].team] += 1;
        }
        for e in sim.events.clone() {
            let (key, team) = match e {
                SimEvent::Kick { player, kind, flux, .. } => (format!("kick:{:?}{}", kind, if flux { "+flux" } else { "" }), sim.players[player].team),
                SimEvent::Goal { team, scorer } => {
                    println!("{:5.1}s GOAL {} by {}", sim.clock, sim.teams[team].def.short_name, sim.players[scorer].name);
                    ("goal".into(), team)
                }
                SimEvent::Tackle { by, success, .. } => (format!("tackle:{}", if success { "won" } else { "lost" }), sim.players[by].team),
                SimEvent::Foul { by, .. } => ("foul".into(), sim.players[by].team),
                SimEvent::Save { keeper, caught } => (format!("save:{}", if caught { "caught" } else { "parried" }), sim.players[keeper].team),
                SimEvent::FluxStart { player, action } => (format!("flux:{}", action.name()), sim.players[player].team),
                SimEvent::Duel { winner, .. } => ("duel won".into(), sim.players[winner].team),
                SimEvent::TeamFlux { team } => ("team flux".into(), team),
                SimEvent::OutOfPlay { restart, team, .. } => (format!("restart:{:?}", restart), team),
                SimEvent::Burnout { player } => ("burnout".into(), sim.players[player].team),
                SimEvent::Sick { player } => ("sick".into(), sim.players[player].team),
                SimEvent::Possession { player } => ("possession".into(), sim.players[player].team),
                _ => continue,
            };
            counts.entry(key).or_default()[team] += 1;
        }
    }
    let el = start.elapsed();
    println!("\nFinal: {} {} - {} {}   ({} ticks in {:.0} ms, {:.1}x realtime)",
        sim.teams[0].def.name, sim.teams[0].score, sim.teams[1].score, sim.teams[1].def.name,
        sim.tick, el.as_secs_f64() * 1000.0, (sim.tick as f64 / 60.0) / el.as_secs_f64());
    let tot = (possession[0] + possession[1]).max(1) as f32;
    println!("possession: {:.0}% / {:.0}%", possession[0] as f32 / tot * 100.0, possession[1] as f32 / tot * 100.0);
    println!("{:<28} {:>6} {:>6}", "event", sim.teams[0].def.short_name, sim.teams[1].def.short_name);
    for (k, v) in &counts {
        println!("{:<28} {:>6} {:>6}", k, v[0], v[1]);
    }
    for t in &sim.teams {
        println!("{} pool {:.0}, strain: {}", t.def.short_name, t.pool,
            t.players.iter().map(|&i| format!("{}={:.0}", sim.players[i].name, sim.players[i].strain)).collect::<Vec<_>>().join(" "));
    }
}
