# Galactik Football

A browser Galactik Football game in Rust with Bevy. PES-style 7-a-side
football with each team's Flux woven into the controls. Two teams at launch:
the Snow Kids (the Breath of Akillian) and the Shadows (the Smog).

See `PLAN.md` for the design and the roadmap. Play it at
<https://rmpr.xyz/galactik/>.

## Layout

| Path                | What                                                         |
|---------------------|--------------------------------------------------------------|
| `crates/gf_core`    | Engine-free deterministic simulation: ball, players, AI, rules, Flux |
| `crates/gf_game`    | Bevy app: input, rendering, HUD, effects                     |
| `assets/teams/*.ron`| Team and player definitions (embedded in the binary)         |
| `web/`              | `index.html`, `Trunk.toml` and CSS for the web build         |
| `dist/`, `dist-gl/` | Built game (committed by CI, WebGPU and WebGL2 variants)     |

## Build and run

```sh
# simulation tests and a headless AI vs AI match
cargo test -p gf_core
cargo run -p gf_core --example headless --release -- <seed> <difficulty>

# native desktop build (fast iteration)
cargo run -p gf_game --release

# web build: needs the wasm32 target and trunk
rustup target add wasm32-unknown-unknown
cargo install trunk
cd web && trunk serve          # http://127.0.0.1:8080
cd web && trunk build --release   # writes ../dist
```

A WebGL2 build for browsers without WebGPU is produced by swapping the
`webgpu` cargo feature for `webgl2` (see `.github/workflows/galactik.yml`).

## Controls

| Action                         | Keyboard (P1)   | Keyboard (P2)   | Gamepad            |
|--------------------------------|-----------------|-----------------|--------------------|
| Move                           | WASD            | Arrows          | Left stick / D-pad |
| Pass / tackle                  | J               | Numpad 1        | A (South)          |
| Shoot / slide                  | K               | Numpad 2        | X (West)           |
| Lob / cross                    | L               | Numpad 3        | B (East)           |
| Through ball                   | I               | Numpad 5        | Y (North)          |
| Sprint                         | Shift           | Numpad 0        | RB                 |
| **Flux modifier (hold)**       | **Space**       | Numpad Enter    | **RT**             |
| Manual aim (hold)              | Ctrl            | Numpad .        | LT                 |
| Switch player                  | Q               | Numpad 4        | LB                 |
| Special (Super Jump / Cloud)   | U               | Numpad 6        | D-pad up / R3      |
| Team Flux (full pool)          | Y               | Numpad 7        | D-pad down / L3    |
| Pause                          | Esc / P         |                 | Start              |

Hold the Flux modifier with an action to turn it into that team's Flux
version of the action:

| Action  | Snow Kids (Breath)                         | Shadows (Smog)                                |
|---------|--------------------------------------------|-----------------------------------------------|
| Sprint  | Breath Burst: 1.5 s of double acceleration | Smog Step: teleport 6 m after a short wind-up |
| Shoot   | Akillian Strike: ×1.5 power, flux shot     | Smog Shot: the ball vanishes for half a second|
| Pass    | Ice Lane: only Flux tackles can intercept  | Veiled Pass: hidden until it arrives          |
| Lob     | High Breath: the receiver super-jumps      | Phase Lob: passes through one defender        |
| Tackle  | Frost Lock: a won tackle freezes the carrier | Smog Snatch: teleport to the carrier and tackle |
| Special | Super Jump header on a high ball           | Eclipse Cloud: 5 m blinding smoke for 3 s     |
| Keeper  | Wall of Ice (automatic)                    | Shadow Keeper teleport (automatic)            |

Each Flux action costs one of three team charges, which fill from play:
the Breath charges on momentum (passes, sprints, shots), the Smog on
disruption (tackles, interceptions, pressing). Every use adds strain to the
player; above 70 they fatigue, above 90 they burn out for the half. The
Smog also makes non-native players (Sinedd) sick. When two Flux actions
collide within 0.4 s a Flux Duel decides who keeps theirs. A full pool
unlocks the Team Flux: Akillian Blizzard or Eclipse.
