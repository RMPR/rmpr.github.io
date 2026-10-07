# Galactik Football — Design & Implementation Plan

A browser-playable Galactik Football game written in Rust with **Bevy** (rendering through
wgpu), with
PES-style football as the base layer and the Flux of each team woven into the
controls rather than bolted on as cutscenes. First release ships two teams:
the **Snow Kids** (the Breath of Akillian) and the **Shadows** (the Smog).

Target quality bar: "not AAA, not far from it" — stylised cel-shaded 3D, 60 fps
on a mid-range laptop in Chrome/Firefox/Safari, full 7v7 match with AI,
couch co-op via two gamepads, no backend.

This document is the plan. Everything in it is a design decision that can be
revisited once a prototype is in hand; the numbers in the stat tables are
starting values for tuning, not canon.

---

## 1. Vision in one paragraph

A 7-a-side futuristic football game. Moment to moment it plays like PES: a
weighty ball, first touches that matter, ground/lob/through passes, sprint
dribbling, manual tackles and a goalkeeper you can control. On top of that,
every player carries their team's **Flux**. Flux is not a separate menu or a
mini-game you enter: it is a modifier held on a trigger that transforms the
action you were already going to do (sprint → Breath burst, shoot → Akillian
strike; sprint → Smog step, tackle → Smog snatch). Flux costs a shared team
resource, carries risk (fouls, exhaustion, Smog sickness), and when two Flux
actions collide the game resolves a short **Flux Duel** in the spirit of
Inazuma Eleven Strikers. The two launch teams play genuinely differently:
Snow Kids are explosive and vertical, Shadows are spatial and deceptive.

---

## 2. Source material (what we are adapting)

### 2.1 The sport as shown in the series
- **Seven players a side**, one of them a goalkeeper.
- Matches are played in large stadia (Genesis Stadium is the iconic one) with
  a holographic look: glowing line markings, floating scoreboards, light walls.
- The **Flux** is a planet-bound energy that enhances speed, strength, agility
  or grants a specific power. Each team's Flux is tied to its home world.
  Flux use is regulated (the Flux Society monitors it; artificial flux and
  flux doping are illegal). We turn "regulated" into in-game rules.
- Overusing a Flux that is not yours hurts you. The Smog makes non-natives ill
  (Aarch, Artegor Nexus and Sinedd all suffered from it): fever, sweating,
  muscle weakness, withdrawal when they stop. This is a gift for game design:
  a built-in risk/reward mechanic.

### 2.2 Snow Kids (Akillian) — the Breath of Akillian
Coach: Aarch. Visual: ice-blue and white kit, the Breath manifests as a blue
white glow / frost trail around the player and the ball.

| # | Player     | Position            | Archetype in-game                                              |
|---|------------|---------------------|----------------------------------------------------------------|
| 1 | Ahito      | Goalkeeper          | Lazy reflexes genius; huge reaction, low stamina, "sleeps" then explodes |
| 2 | Thran      | Defender            | Tactician; best positioning/interception, average pace         |
| 3 | Mei        | Defender            | Fast, precise, good in the air                                 |
| 4 | Rocket     | Midfielder, captain | Complete midfielder; vision, passing, highest Flux control     |
| 5 | Tia        | Midfielder          | Technical playmaker; best Breath mastery, dribbling            |
| 6 | Micro-Ice  | Striker             | Small, agile, tricks, quick finishes                           |
| 7 | D'Jok      | Striker             | Star striker; power shots, pace, big Flux output, ego          |
| 8 | Mark       | Sub (Mid/Def)       | Season 2 arrival; solid all-rounder                            |
| 9 | Yuki       | Sub (GK)            | Season 2 arrival; backup keeper, agile                         |
|10 | Sinedd     | Sub (Striker)       | Season 1 / season 3; aggressive striker (see also Shadows)     |

Breath on the pitch: explosive acceleration, super-jumps, power in strikes,
and a visible blue "wake" the opposing team can read.

### 2.3 Shadows (Shadow Archipelago) — the Smog
Coach: Artegor Nexus. Visual: black/dark-violet kit, red eyes, the Smog is a
cloud of black smoke that envelops player and ball.

| # | Player   | Position              | Archetype in-game                                   |
|---|----------|-----------------------|-----------------------------------------------------|
| 1 | Senex    | Goalkeeper            | Tall, teleporting keeper; can Smog-step across goal |
| 2 | Zed      | Defender              | Physical stopper, Smog snatch specialist            |
| 3 | Cron     | Defender              | Fast reader, intercepts out of the fog              |
| 4 | Nihlis   | Midfielder            | Pass-and-vanish midfielder                          |
| 5 | Nilli    | Midfielder            | Pressing, high stamina                              |
| 6 | Paul     | Second striker        | Poacher who appears behind the line                 |
| 7 | Fulmugus | Striker, captain      | Powerful, intimidating; long range                  |
| 8 | Sinedd   | Striker (human)       | Deadly but **not native**: Smog sickness applies    |

Smog on the pitch: strength/speed boost, **short-range teleport**, and the
smoke cloud can temporarily **blind** opponents nearby. Non-natives risk Smog
sickness.

---

## 3. Game design

### 3.1 Match rules
- 7v7, a pitch of **60 m × 40 m** (futsal-ish ratio scaled up), goals 5 m × 2 m.
- Two halves, default **5 real minutes** each (configurable 3/5/8/10). The
  displayed clock counts 0–45 per half at an accelerated rate purely for
  flavour; all rules use real time.
- No offside by default (matches the show and keeps 7v7 flowing). Toggle in
  settings.
- Ball out over the touchline → **re-entry kick** (kicked, not thrown; faster
  restart). Over the goal line → goal kick / corner.
- Fouls → free kick; in the box → penalty. Flux-enhanced tackle from behind is
  always a foul and an automatic yellow ("Flux Society ruling").
- Subs: up to 3, from the bench, at any dead ball.
- Extra time + penalties available for cup matches.

### 3.2 Controls (PES-style)
Gamepad first, keyboard equivalent. Two gamepads for local versus.

| Action                      | Gamepad                     | Keyboard            |
|-----------------------------|-----------------------------|---------------------|
| Move player                 | Left stick                  | WASD                |
| Short/ground pass           | A (×)                       | J                   |
| Lob / cross                 | B (○)                       | L                   |
| Through ball                | Y (△)                       | I                   |
| Shoot (hold = power)        | X (□)                       | K                   |
| Sprint                      | RB (R1)                     | Shift               |
| **Flux modifier (hold)**    | **RT (R2)**                 | **Space**           |
| Manual modifier             | LT (L2)                     | Ctrl                |
| Switch player / tackle      | LB (L1)  / A when defending | Q / J               |
| Slide tackle                | X when defending            | K                   |
| Call teammate press         | RB when defending           | Shift               |
| GK rush                     | Y when defending            | I                   |
| Super cancel                | RB + RT                     | Shift + Space       |
| Tactics quick-switch        | D-pad                       | 1-4                 |
| Pause / menu                | Start                       | Esc                 |

Right stick: skill moves (feints, step-overs) while dribbling; aim lift on
shots when Manual is held.

### 3.3 Base football model ("the PES layer")
Deterministic fixed-step simulation at 60 Hz, independent from rendering.

**Ball.** Point mass with radius 0.11 m. Gravity, air drag, Magnus curve
from spin, ground friction, restitution 0.6 on the pitch, 0.4 off posts.
Spin is set by the kick (foot side + aim + Flux). The ball is "owned" by a
player only while within a control radius and the player's control stat beats
the ball's relative speed.

**Player.** Capsule 0.4 m radius, 1.8 m tall. Motion = acceleration-limited
steering toward input direction, with turn rate limited by speed (a sprinting
player cannot turn sharply, the classic PES feel). Stats drive the constants.

**First touch.** On receiving, the ball is pushed `touch_error` metres away
in a random direction scaled by (incoming speed / control stat). Good players
keep the ball glued; bad players at speed let it bounce off.

**Passing.** Short pass: ground, speed from hold time, target = best teammate
in a cone around the stick direction (PES "assisted"), Manual (LT) = raw
direction and power. Through ball: ground pass aimed into space ahead of a
teammate's run. Lob: aerial pass with apex scaled by power.

**Shooting.** Power from hold (0.15–1.0 s). Accuracy cone widens with power,
with body angle to goal, with ball height, and shrinks with the shooting stat.
Finesse (Manual + shoot) trades power for curl.

**Dribbling.** Ball sits ahead at 0.5 m walking, 1.5 m sprinting. Sprint
knocks the ball further; a defender inside that gap can steal. Right-stick
feints move the ball laterally and freeze a defender's AI for 300 ms if timed.

**Defending.** Jockey (no button), press (A hold = auto-run at ball carrier),
standing tackle (A tap in range), slide (X), teammate press (RB). Tackle
success = `tackle` vs `ball_control` with an angle bonus (from the front) and
a timing window. Fouls use the same roll with a "late" factor.

**Goalkeeper.** AI-driven unless manually controlled. Positioning on the
ball-goal line, reaction time, dive reach from stats, parry vs catch decided
by shot power.

**Stats (0–99).** `pace, acceleration, ball_control, dribbling, short_pass,
long_pass, shooting, heading, tackle, strength, stamina, reactions, gk_reflex,
gk_reach, flux_power, flux_control`. Per-player values live in RON data
files, see Appendix A.

### 3.4 Team AI
- Formation anchors per phase (attack / defend / transition) stored per team,
  e.g. Snow Kids 2-2-2 attacking, Shadows 3-1-2 counter.
- Each AI player picks a behaviour via a utility score: keep position, make a
  run, support carrier, press carrier, cover passing lane, mark, shadow.
- Carrier AI: scored options (pass/dribble/shoot/clear) with risk weighting
  that depends on team "mentality" (D-pad: defensive / balanced / attacking /
  all-out).
- AI uses Flux with the same economy as the human; team personalities set the
  thresholds (Snow Kids spend Flux in attack; Shadows hoard it for transitions).
- Difficulty scales reaction delays, pass error, and Flux aggressiveness.

### 3.5 The Flux layer — the "clever" part

Design goals:
1. Flux never takes you out of the flow (no menu, no battle screen longer
   than ~1 s).
2. Each team's Flux changes **how you play**, not just how big your numbers are.
3. Flux is a resource with risk, so matches have an arc (hoard vs spend).
4. Opponents can **read and answer** Flux — it is not a free win button.

#### 3.5.1 Flux as a modifier
Hold **RT** and the next action becomes a Flux action. The same button layout
gives different results per team:

| Base action    | Snow Kids — Breath                                     | Shadows — Smog                                                 |
|----------------|--------------------------------------------------------|----------------------------------------------------------------|
| Sprint         | **Breath Burst**: 1.5 s acceleration ×2, frost trail  | **Smog Step**: teleport 6 m in stick direction, 0.3 s wind-up  |
| Shoot          | **Akillian Strike**: power ×1.6, ball glows, knocks GK back on parry | **Smog Shot**: ball disappears in smoke for 0.5 s mid-flight; GK reaction penalty |
| Pass / through | **Ice Lane**: ball slides frictionless on an ice trail, cannot be intercepted by non-Flux tackles | **Veiled Pass**: pass target hidden from AI/opponent HUD until ball emerges |
| Lob            | **High Breath**: super-jump header target — receiver leaps 3 m | **Phase Lob**: ball passes *through* a defender's hitbox once      |
| Tackle         | **Frost Lock**: successful tackle also freezes the carrier for 0.6 s | **Smog Snatch**: teleport 4 m to the carrier and tackle in one motion |
| Jump / header  | **Super Jump**: 3 m vertical, aerial duels auto-win vs non-Flux | **Eclipse Cloud**: deploy a 5 m smoke cloud that blinds non-Shadows (their input is dampened, camera fogs) |
| GK             | **Wall of Ice**: 1 s expanded save box                 | **Shadow Keeper**: teleport to the shot's predicted line        |

#### 3.5.2 Flux economy
- Each **team** has a shared **Flux Pool** of 0–300 shown as three charges.
- Each **player** has a **Resonance** value 0–100 that multiplies the effect
  of their Flux actions (`flux_power`) and reduces cost (`flux_control`).
- The Pool charges through *style-specific* play:
  - Breath charges on **momentum**: successful passes in sequence, sprints
    with the ball, shots on target, dribbles past a defender.
  - Smog charges on **disruption**: interceptions, tackles, pressing the
    carrier within 3 m for 2 s, keeping possession in your own half, being
    behind on the scoreboard (comeback energy).
- A basic Flux action costs one charge (100). A signature (see below) costs
  two. The captain's Team Flux costs all three.
- Pool slowly leaks when full to encourage spending.

#### 3.5.3 Flux strain and sickness (the risk)
- Every Flux action adds **strain** to the player who performed it.
- Strain above 70 → **Flux fatigue**: stamina drain ×2, Flux actions weaker.
- Strain above 90 → the player is **burned out** for the rest of the half:
  no Flux, reduced pace.
- Shadows specific: **non-native** players (Sinedd) accrue strain at ×1.5 and
  at >70 get **Smog sickness** (random stumbles, shaking aim). Natives never
  get sick. This makes Sinedd a high-risk high-reward pick.
- Snow Kids specific: the Breath is "clean" but **cold burst stamina**:
  a Burst costs a stamina chunk immediately, so spamming kills your legs.
- Strain recovers slowly; full reset at half time; a substitution clears it.

#### 3.5.4 Flux duels (the Inazuma moment)
When a Flux action is met by a Flux counter within a 0.4 s window — e.g. an
Akillian Strike vs a Shadow Keeper, a Smog Snatch vs a Frost Lock dribble —
the game enters a **Flux Duel**: time slows to 0.2× for ~0.8 s, the camera
cuts to a tight shot, both players' Flux visuals flare, and the outcome is
`attacker_power × timing_bonus` vs `defender_power × timing_bonus` where
timing bonus comes from how close to the duel-trigger instant each side
pressed. The loser's Flux fizzles (and they take extra strain); the winner's
action completes at full strength. No extra button prompts beyond the one
already pressed, so the duel never feels like a QTE.

#### 3.5.5 Signature moves (per player, cost two charges)
Hold RT + the action for a full charge-up (1 s) to unleash a signature.
Launch set, one per starter:

Snow Kids
- D'Jok — **Comet Strike**: shot that curves twice; parried balls rebound to the shooter.
- Micro-Ice — **Snowflake Spin**: 360° dribble that places three decoy frost clones for 1 s.
- Rocket — **Captain's Call**: all teammates Breath-burst toward space for 2 s (team-wide run trigger).
- Tia — **Glacier Pass**: through ball that curves around any defender in the lane.
- Thran — **Ice Wall**: 4 m frozen barrier that stops ground passes for 2 s.
- Mei — **Blizzard Clear**: header that travels 40 m with pinpoint accuracy.
- Ahito — **Dreamcatch**: 1.5 s during which any shot on target is caught, then Ahito "falls asleep" (−30 reactions for 20 s).

Shadows
- Fulmugus — **Black Sun**: long-range shot that blinds the keeper for 0.8 s.
- Sinedd — **Shadow Fang**: teleport past the last defender and shoot in one input (sickness ×2 strain).
- Paul — **Phantom Run**: becomes invisible on the HUD and to AI for 2 s.
- Nihlis — **Smoke Signal**: pass that leaves a smoke lane; the receiver can Smog Step along it for free.
- Nilli — **Tireless Press**: 4 s where every tackle attempt is a Smog Snatch at no cost.
- Zed — **Chokehold**: successful tackle also drains one opponent Flux charge.
- Cron — **Dark Reading**: 3 s where all opponent pass targets are revealed and intercept radius doubles.
- Senex — **Night Gate**: the goal mouth fills with smoke for 1.5 s, all shots lose 50% accuracy.

#### 3.5.6 Team Flux (captain, all three charges)
- Snow Kids — **Akillian Blizzard**: 10 s where every Snow Kid has free basic
  Breath actions and the pitch frosts over (ball friction drops for everyone).
- Shadows — **Eclipse**: 10 s where the stadium darkens, opponents see only a
  spotlight around the ball, and Smog Steps cost nothing.

Both come with a stadium-wide VFX state and a music stinger, which is the
"Strikers" spectacle moment without stopping play.

### 3.6 Modes (launch)
1. **Friendly**: Snow Kids vs Shadows, any side, vs AI or local 2P.
2. **Cup**: best-of-three series with persistent strain between legs (subs matter).
3. **Training**: free play, Flux tutorial prompts.
4. **Options**: difficulty, half length, offside, assist levels, key bindings.

### 3.7 Presentation
- **Look**: cel-shaded characters with outline pass, flat-lit stadium with
  emissive holographic markings, bloom for Flux. Match the series' CG look
  rather than photorealism.
- **Camera**: PES "wide" broadcast camera as default, dynamic zoom on ball
  speed, duel camera cut, goal replay from 2 fixed angles (deterministic sim
  makes replays free: re-simulate from a snapshot).
- **UI**: minimal — clock, score, Flux charges per team, strain pips on the
  controlled player, radar.
- **Audio**: crowd bed, kick/bounce foley, per-Flux whooshes, duel stinger,
  announcer-free (voices are out of scope).

---

## 4. Technical architecture

### 4.1 Stack (versions as of October 2026)
| Concern            | Choice                                                                 |
|--------------------|------------------------------------------------------------------------|
| Language           | Rust stable, edition 2024                                               |
| Engine             | **Bevy 0.19.x** (0.20 is at release candidate; migrate once stable). Bevy renders through wgpu, so the browser story is WebGPU first with a WebGL2 build via Bevy's `webgl2` feature |
| Math               | `glam` (re-exported by Bevy)                                            |
| ECS / scheduling   | Bevy ECS. The football simulation runs as **one chained system set** in `FixedUpdate` so ordering is explicit and deterministic |
| Rendering          | Bevy PBR pipeline with a cel-shading `ExtendedMaterial` on top of `StandardMaterial`, built-in bloom, built-in skinned meshes, Fullscreen Material for the outline post pass |
| Animation          | `bevy_animation` + `bevy_gltf` with `AnimationGraph` for blend trees and one-shot layers |
| Particles          | `bevy_hanabi` 0.19 (GPU particles for frost and smoke; falls back to CPU-side mesh instancing on WebGL2 where compute is unavailable) |
| Input              | Bevy's `ButtonInput`/`Gamepads` wrapped by `bevy_enhanced_input` 0.26 for action maps, rebinding and per-player contexts |
| Audio              | `bevy_audio` (built in, vorbis). Swap for `bevy_kira_audio` only if mixing buses prove necessary |
| Menus / debug UI   | `bevy_ui` for in-game HUD and menus; `bevy_egui` 0.42 for the tuning panel and dev tools; `bevy_dev_tools` for FPS overlay |
| Data               | RON via `ron` + `serde`, loaded through Bevy's `AssetServer` with a custom `AssetLoader` for team and Flux files |
| Build (web)        | `trunk` 0.21.x with `wasm-bindgen`; `wasm-opt -Oz` in release                 |
| Build (native)     | plain `cargo run` with Bevy's `dynamic_linking` feature for fast iteration    |
| Tests              | `cargo test` on `gf_core`; headless Bevy `MinimalPlugins` app for integration and determinism tests |

Native desktop build is kept working at all times — it is the fast iteration
path, the debugger path, and where the tuning panel lives.

### 4.2 Workspace layout
```
galactik/
  Cargo.toml                 # workspace
  crates/
    gf_core/                 # pure game simulation: NO bevy dependency except glam
      src/ball.rs, player.rs, pitch.rs, rules.rs, ai/, flux/, input.rs, sim.rs
    gf_data/                 # RON schema types for teams, players, Flux actions
    gf_game/                 # Bevy app: plugins below
      src/
        main.rs              # App builder, platform setup, state machine (Title/Menu/Match/Replay)
        sim_plugin.rs        # FixedUpdate: gather InputFrames -> gf_core::Sim::step -> write back
        view_plugin.rs       # Update: interpolate sim state into Transforms, camera, animation drive
        render/              # cel material, outline fullscreen material, holo pitch shader, bloom config
        vfx/                 # hanabi effect assets for Breath and Smog, duel camera, Team Flux stadium state
        audio_plugin.rs
        ui/                  # HUD (bevy_ui), menus, egui tuning panel (native/dev only)
        assets_plugin.rs     # RON asset loaders, loading state, lazy team loading
  assets/
    teams/snow_kids.ron, teams/shadows.ron
    flux/breath.ron, flux/smog.ron
    models/*.glb, textures/*.png, anim/*.glb, audio/*.ogg, fonts/*.ttf
    shaders/*.wgsl
  web/
    index.html, Trunk.toml, style.css
  PLAN.md                    # this file
```

`gf_core` stays engine-free and deterministic given an input stream. Bevy
never owns game state: the ECS holds a single `Res<MatchSim>` resource
wrapping `gf_core::Sim`, plus view-side entities (meshes, animation players,
particles) that mirror it. That keeps replays, regression tests of the Flux
rules and later rollback netplay possible, and it means swapping engines
again would cost one crate, not the game.

### 4.3 Main loop (Bevy schedules)
```
FixedUpdate  @ 60 Hz  (Time<Fixed>)         Update (render rate)
  gather_inputs      -> Vec<InputFrame>       interpolate_view   (prev/curr sim state, Time<Fixed>::overstep_fraction)
  step_sim           -> MatchSim.step()       drive_animation    (locomotion speed/dir, one-shot events)
  emit_sim_events    -> EventWriter<SimEvent> camera_follow      (broadcast cam, duel cut)
                                              hud_update
```
The three `FixedUpdate` systems are `.chain()`ed so they are never
reordered or parallelised. `SimEvent` (kick, goal, foul, flux_start,
duel, ...) is the only channel from sim to presentation: VFX, audio, camera
and HUD all react to events, never read sim internals directly. The
`step_sim` system is pure and takes an input slice, so a replay is the
same system fed from a file.

### 4.4 Renderer
- Bevy's standard 3D pipeline (HDR, tonemapping, built-in bloom for Flux
  emissives, MSAA 4× on native, FXAA on web).
- **Cel shading** as an `ExtendedMaterial<StandardMaterial, CelExtension>`:
  a small WGSL fragment that quantises the lighting term into 2–3 bands and
  adds a rim light tinted by the player's Flux colour. One material type for
  players, kit and stadium, with per-instance uniforms for flux tint and
  intensity.
- **Outlines** via a Fullscreen Material post pass (depth and normal edge
  detection); inverted hull as the WebGL2 fallback.
- Holographic pitch and side walls: unlit emissive materials with a scrolling
  grid texture and fresnel alpha.
- One directional light with cascaded shadow maps (2 cascades), ambient from
  a flat environment colour.
- Crowd: instanced billboards (Bevy auto-instances identical mesh/material).
- VFX: `bevy_hanabi` effects (frost trail, Breath burst ring, Smog puff,
  Eclipse fog volume as a big soft particle cloud), ball ribbon trail via a
  dynamic mesh.
- Resolution scale through the camera's `Viewport` and a `RenderTarget`
  with upscale on slow frames.

### 4.5 Animation
- Shared humanoid rig for all 18 players; clips loaded from glTF through
  `bevy_gltf` into one `AnimationGraph` per player entity.
- Blend nodes: locomotion (idle/walk/jog/sprint × 8-direction strafe) driven
  by sim velocity; additive upper-body layer for looking at the ball; one-shot
  nodes for kicks, tackles, headers, dives, Flux poses, celebrations, each
  triggered by a `SimEvent`.
- Foot-to-ball alignment is a small transform tweak on the kicking foot
  bone in `drive_animation`; no IK solver at launch.

### 4.6 Physics
Hand-rolled inside `gf_core` exactly as before: ball integrator with Magnus,
capsule separation, kick contacts, posts as cylinders. No `avian` or
`bevy_rapier`: the pitch is a plane, the entities are capsules and one
sphere, and determinism across browsers matters more than a general solver.
Bevy only ever receives resulting positions.

### 4.7 Input
`bevy_enhanced_input` action contexts: `OnPitch`, `Defending`, `Menu`. One
context instance per local player bound to a gamepad id or the keyboard.
Each tick `gather_inputs` reads the action values into an `InputFrame`
struct per slot; AI fills the remaining slots. The sim never sees Bevy input
types.

### 4.8 Audio
`bevy_audio` with vorbis assets. Three logical groups (music, SFX, crowd)
with volumes stored in a settings resource. Web autoplay rules require a
user gesture before the audio context starts; the title screen's
"Press any key" handles it. Flux sounds are triggered from `SimEvent`s.

### 4.9 Web deployment inside this Jekyll site
- The game lives in `galactik/` of this repo and is published at
  `https://rmpr.xyz/galactik/`.
- `trunk build --release` outputs to `galactik/dist/`. A GitHub Actions
  workflow builds the wasm on push to `master`, runs `wasm-opt -Oz`, and
  commits `galactik/dist/` (or uploads a Pages artifact if the site moves to
  the Actions-based Pages deploy). Jekyll copies any non-underscore folder as
  static files, so `dist/` is served as-is.
- Add `galactik/crates`, `galactik/assets`, `galactik/target`,
  `galactik/Cargo.*` to the Jekyll `exclude` list; only `galactik/dist` ships.
- Release profile: `opt-level = "z"`, `lto = "fat"`, `codegen-units = 1`,
  `panic = "abort"`, Bevy with `default-features = false` and only the
  feature collections needed (`3d`, `ui`, `animation`, `bevy_gltf`,
  `bevy_audio`, `vorbis`, `png`, `webgpu`/`webgl2`). Expected wasm:
  **12–18 MB raw, 4–6 MB gzipped**, which GitHub Pages serves compressed.
  Budget: ≤ 20 MB raw wasm, ≤ 25 MB assets, teams and stadium loaded after
  the title screen behind Bevy's `LoadingState`.
- No threads at launch: GitHub Pages cannot send the COOP/COEP headers that
  `SharedArrayBuffer` needs, so Bevy runs its schedule single-threaded on
  wasm (this is its default there).
- Two builds: `--features webgpu` as the default at `galactik/`, and a
  `--features webgl2` build at `galactik/gl/`. The loader page checks
  `navigator.gpu` and picks one. Hanabi compute effects and cascaded shadows
  are disabled by a `GpuCaps` resource on the WebGL2 build.
- Bevy's asset loading on the web goes over `fetch`; keep assets under
  `galactik/dist/assets/` and set `AssetPlugin::file_path` accordingly.

---

## 5. Assets (personal use: lean on what the internet already has)
- **Reference**: the Galactik Football wiki on Fandom for faces, kits, team
  logos and the Genesis Stadium; the series itself for animation reference of
  Flux effects (blue frost wake, black smoke bursts).
- **Characters**: one stylised humanoid base mesh (Quaternius "Universal
  Base" or Kenney's animated character packs, both CC0) retargeted to Mixamo
  animations (free: run, sprint, kicks, slide tackle, header, keeper dives,
  celebrations). Search Sketchfab for fan-made Galactik Football models and
  use any that are downloadable; otherwise build each player from the base
  mesh with hair/skin/kit textures (D'Jok's red hair, Micro-Ice's blue tuft,
  Shadows' grey skin and red eyes are mostly texture and colour work).
- **Stadium**: modelled in Blender from the Genesis Stadium reference; mostly
  emissive materials and simple geometry, so cheap to make.
- **Audio**: freesound.org CC0 foley, a crowd loop, synthesised Flux sounds;
  no series music.
- Keep an `assets/ATTRIBUTION.md` listing every source and licence.

---

## 6. Milestones

Each milestone ends with a runnable build both native and in the browser.
Rough effort assumes evenings/weekends.

| # | Milestone              | Deliverable                                                                                  | Est.   |
|---|------------------------|----------------------------------------------------------------------------------------------|--------|
| 0 | Skeleton               | Workspace, Bevy app with a lit cube on native and `trunk serve`, GitHub Pages deploy at `/galactik/`, CI with wasm-opt, size report | 1 wk |
| 1 | Pitch & ball           | Broadcast camera, holographic pitch material, `gf_core` ball physics driven from `FixedUpdate`, kicks from a capsule "player", debug overlay, deterministic sim tests | 2 wk |
| 2 | One player PES feel    | Movement model, dribbling, sprint, first touch, shooting, keeper AI; egui tuning panel with live sliders over `gf_core` constants | 3 wk |
| 3 | Full 7v7               | Passing types, tackles, fouls, dead balls, team AI formations and roles, player switching, clock, score, 2P local via input contexts | 4 wk |
| 4 | Flux core              | Flux pool, strain, modifier mapping for both teams' basic Flux actions, duels, Flux VFX v1 with hanabi | 3 wk |
| 5 | Characters & animation | Shared rig in glTF, `AnimationGraph` locomotion + one-shots, 18 player looks, cel material + outline pass, stadium model | 2 wk |
| 6 | Signatures & Team Flux | All 15 signatures, Akillian Blizzard, Eclipse, duel camera, replays, audio | 3 wk |
| 7 | Modes & polish         | `bevy_ui` menus, Cup mode, options and rebinding, difficulty, WebGL2 build and `GpuCaps`, loading screen, bug bash | 3 wk |
| 8 | Release 1.0            | Published at rmpr.xyz/galactik with a blog post; tag `v1.0`                                    | —      |

Milestone 5 shrinks from four weeks to two compared with the raw-wgpu plan:
glTF skinning, the animation graph and bloom come with Bevy. Milestone 0 and
7 absorb the extra web-build work (feature trimming, size, two builds).

Later (not planned in detail): more teams (Wambas, Xenons, Rykers, Pirates,
Lightnings, Cyclops, Elektras), online play via rollback (the deterministic
core is built for it), mixed-flux "Paradisia" mode.

### Definition of done per milestone
- Native and web builds run; web build ≥ 60 fps on an integrated-GPU laptop
  in Chrome and Firefox, ≥ 30 fps on the WebGL2 path.
- `cargo test` green; a 10-minute recorded input replay re-simulates to the
  same final state hash (determinism guard), run in a headless
  `MinimalPlugins` app in CI.
- `cargo clippy -- -D warnings` clean.
- Release wasm size printed in CI and under budget.

---

## 7. Key risks and the plan for each
| Risk | Mitigation |
|------|------------|
| Wasm size on a static host | Trim Bevy features from day one (M0 prints the size), `opt-level = "z"`, `wasm-opt -Oz`, lazy team/stadium loading; if it still exceeds 20 MB raw, drop `bevy_hanabi` on web for a hand-rolled instanced particle mesh |
| Bevy breaking releases every ~3–4 months | Pin to 0.19 for the whole build; migrate only at milestone boundaries, and only when `bevy_hanabi`, `bevy_egui` and `bevy_enhanced_input` have all caught up |
| Determinism inside an ECS scheduler | Sim lives in one `Res<MatchSim>` stepped by one chained system; no sim logic in queries; `Time<Fixed>` only. The determinism test guards it |
| Character art pipeline is still the biggest art time sink | Shared rig + Mixamo clips; placeholder capsules through M4; 18 looks are texture variants of one mesh |
| Flux feels like a win button | Duels, strain and the Smog sickness are in M4, before any art polish, so balance is tuned on gameplay alone |
| WebGL2 fallback drags the renderer down | `GpuCaps` resource gates compute particles, cascades and the fullscreen outline; the fallback is inverted hull and CPU particles |
| Floating-point determinism across browsers | Sim avoids platform libm `sin/cos` in hot paths (own approximations), never reads wall-clock; tests hash state each tick |
| Scope creep into "all 8 teams" | Launch is two teams by decision; the data-driven Flux tables make more teams additive, not structural |

---

## 8. Decisions already made (so they are not re-argued later)
1. **Bevy 0.19** as the engine, chosen for time to first playable (glTF,
   skinning, animation graph, bloom, UI and the web build all come for
   free) and accepting a larger wasm binary in exchange.
2. The simulation (`gf_core`) is engine-free and deterministic from day one,
   and Bevy never owns match state. It is stepped by one chained system in
   `FixedUpdate` and talks to presentation only through `SimEvent`s.
3. Flux is a held modifier on existing actions; no separate command menu.
4. Team Flux economy is shared, strain is per player.
5. No offside by default; 7v7 on a 60×40 m pitch.
6. Cel-shaded look matching the series, not realism.
7. Two teams at launch; data-driven so more teams are content, not code.
8. Pin Bevy and its plugins for the whole 1.0 cycle; migrate at milestone
   boundaries only.

---

## Appendix A — Data file sketch (`assets/teams/snow_kids.ron`)
```ron
Team(
    id: "snow_kids",
    name: "Snow Kids",
    home: "Akillian",
    coach: "Aarch",
    flux: "breath",
    kit: Kit(primary: "#CFE8FF", secondary: "#1E4D8C", accent: "#FFFFFF"),
    formation: Formation(attack: "2-2-2", defend: "3-2-1"),
    captain: "rocket",
    team_flux: "akillian_blizzard",
    players: [
        Player(id: "ahito", name: "Ahito", number: 1, pos: GK,
            stats: Stats(pace: 55, acceleration: 60, ball_control: 50, dribbling: 40,
                short_pass: 55, long_pass: 60, shooting: 30, heading: 50, tackle: 40,
                strength: 60, stamina: 45, reactions: 92, gk_reflex: 90, gk_reach: 84,
                flux_power: 70, flux_control: 75),
            signature: "dreamcatch"),
        Player(id: "thran", name: "Thran", number: 2, pos: DF,
            stats: Stats(pace: 68, acceleration: 66, ball_control: 70, dribbling: 60,
                short_pass: 78, long_pass: 74, shooting: 55, heading: 72, tackle: 84,
                strength: 74, stamina: 80, reactions: 86, gk_reflex: 0, gk_reach: 0,
                flux_power: 65, flux_control: 80),
            signature: "ice_wall"),
        Player(id: "mei", name: "Mei", number: 3, pos: DF, /* ... */ signature: "blizzard_clear"),
        Player(id: "rocket", name: "Rocket", number: 4, pos: MF, /* ... */ signature: "captains_call"),
        Player(id: "tia", name: "Tia", number: 5, pos: MF, /* ... */ signature: "glacier_pass"),
        Player(id: "micro_ice", name: "Micro-Ice", number: 6, pos: ST, /* ... */ signature: "snowflake_spin"),
        Player(id: "djok", name: "D'Jok", number: 7, pos: ST, /* ... */ signature: "comet_strike"),
        Player(id: "mark", name: "Mark", number: 8, pos: MF, /* bench */),
        Player(id: "yuki", name: "Yuki", number: 9, pos: GK, /* bench */),
        Player(id: "sinedd", name: "Sinedd", number: 10, pos: ST, /* bench */ native: false),
    ],
)
```

`assets/flux/smog.ron` describes the Smog as a list of `FluxAction` records
keyed by base action, each with cost, strain, wind-up, duration, VFX id and a
`Effect` enum variant (`Teleport{dist}`, `Cloud{radius, blind_secs}`,
`HideBall{secs}`, `PhaseThroughOnce`, ...). The Breath uses the same schema
with different variants (`Burst{mult, secs}`, `IceLane{secs}`,
`SuperJump{height}`, `Freeze{secs}`, ...). Adding a third team is adding a
RON file and, when a genuinely new effect is needed, one enum variant.

## Appendix B — Flux state machine (per player, in `gf_core::flux`)
```
Idle ──(RT held + action)──▶ WindUp(action, t)
WindUp ──(t ≥ wind_up)──▶ Active(action, t)          # effect applied, pool debited, strain added
WindUp ──(tackled / RT released early)──▶ Idle        # cost refunded 50%
Active ──(opponent Flux within 0.4 s)──▶ Duel(vs)
Duel ──(resolve)──▶ Active (won) | Fizzle (lost)
Active ──(t ≥ duration)──▶ Cooldown(0.5 s) ──▶ Idle
```
Strain and sickness are evaluated at the Active transition; Burnout and
Smog sickness are status effects on the player entity checked by movement
and aim code.

## Appendix C — Workspace `Cargo.toml` sketch
```toml
[workspace]
members = ["crates/gf_core", "crates/gf_data", "crates/gf_game"]
resolver = "3"

[workspace.dependencies]
bevy = { version = "0.19", default-features = false }
glam = { version = "0.30", features = ["serde"] }   # match the glam Bevy 0.19 re-exports
ron = "0.12"
serde = { version = "1", features = ["derive"] }
bevy_hanabi = { version = "0.19", default-features = false, features = ["3d"] }
bevy_enhanced_input = "0.26"
bevy_egui = "0.42"
log = "0.4"

[profile.dev]
opt-level = 1
[profile.dev.package."*"]
opt-level = 3

[profile.release]
opt-level = "z"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

`crates/gf_game/Cargo.toml`:
```toml
[dependencies]
bevy = { workspace = true, features = [
    "3d", "ui", "animation", "bevy_gltf", "bevy_audio", "vorbis", "png",
    "bevy_state", "bevy_window", "bevy_winit", "multi_threaded",
] }
gf_core = { path = "../gf_core" }
gf_data = { path = "../gf_data" }
bevy_hanabi.workspace = true
bevy_enhanced_input.workspace = true
ron.workspace = true
serde.workspace = true

[features]
default = ["webgpu"]
webgpu = ["bevy/webgpu"]
webgl2 = ["bevy/webgl2"]
dev = ["bevy/dynamic_linking", "bevy/bevy_dev_tools", "dep:bevy_egui"]

[dependencies.bevy_egui]
workspace = true
optional = true

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
console_error_panic_hook = "0.1"
```
`gf_core` depends on `glam`, `serde` and `gf_data` only, never on `bevy`.

## Appendix D — Sources consulted
- Galactik Football wiki (Fandom): Snow Kids, The Shadows, The Smog, Sinedd.
- Wikipedia: Galactik Football (7-a-side, Flux, seasons overview).
- Inazuma Eleven Strikers reviews for the gauge / special-move structure.
- PES 2021 controls guides for the control scheme and super cancel.
- crates.io for `bevy` 0.19.1, `bevy_hanabi` 0.19.0, `bevy_egui` 0.42.0,
  `bevy_enhanced_input` 0.26.0, `trunk` 0.21.14 (Oct 2026).
- Bevy 0.18 release notes (Fullscreen Materials, feature collections) and
  the 0.18 → 0.19 migration guide.
