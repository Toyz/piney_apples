# Modern rendering and sound

After the clean-up (plans/rust-cleanup.md). Today the picture and the sound
are made the PS2's way: the game builds a GS-shaped frame each frame
(`piney-draw`: primitives with GS blend and test state, model draws with
VU1-style lighting and fog worked out on the CPU, frame-buffer copies for
blur, water and feedback, a 512 x 448 target scaled up), and `piney-gs`
draws it as the GS would. The sound is the IOP's: a MIDI sequencer, the
hardware synthesizer and an SPU2 model mixing 24 voices at 48 kHz with its
4-point Gaussian interpolation and its reverb, behind one mutex the cpal
callback holds for a whole buffer.

That made the port checkable: what the game asks for (the draw list, the
sound commands) is compared with the game's own code. It is also why the
picture and the sound cost what they do. This phase keeps that contract
and replaces what sits behind it with how modern engines render and mix.

## The rule

- The game's state stays bit-exact. Only presentation changes: nothing the
  renderer or the mixer does may feed back into the game.
- The checks move to the contract: the draw list and the sound commands
  stay compared with the game's code; pictures and audio are checked by
  their own tests (golden shots per mode, signal tests).
- A Classic mode stays: today's GS and SPU2 paths, kept as the reference
  and as an option for players who want the original look and sound
  (the maintainer's decision, 2026-10-29).
- The modern path aims at the faithful look and sound at modern quality:
  the same art, lighting and mix, sharper and smoother. Enhancements
  (real shadows, new lighting, post effects, a modern reverb) are opt-in
  extras, never the default (the maintainer's decision, 2026-10-29).

## Crates: a core per system, a crate per backend

Each output system gets a core crate that holds its contract and a crate
for each way of producing it (the maintainer's decision, 2026-10-29). The
game depends only on the cores; which backend runs is a setting.

| system | core (the contract) | Classic | Modern |
| --- | --- | --- | --- |
| picture | `piney-render`: today's `piney-draw` frame types (what the game asks to draw) plus a `Renderer` trait (`begin`, `submit(&Frame)`, `present`, `read_back`, `set_overlay`, resize and options) and the shared asset ids | `piney-render-classic`: today's `piney-gs`, the GS drawn as the GS did | `piney-render-modern`: retained GPU resources, shader skinning and lighting, the render graph |
| sound | `piney-sound`: everything checked against the game: the EE sound driver, the MIDI sequencer, the synthesizer's note, envelope and pitch logic, `se3d`, the voice and stream commands; its output is the SPU2 register-write stream plus the stream and voice PCM requests; an `AudioBackend` trait and the device output (cpal) | `piney-sound-classic`: the SPU2 model (24 voices, Gaussian interpolation, the SPU2 reverb) consuming those register writes | `piney-sound-modern`: the same writes read as voice events (key on and off, pitch, ADSR, volume, sample address) into a modern mixer with decoded PCM, buses, resampling and spatial audio |

- The cut lies where the checks against the game already stop: the frame
  for the picture, the SPU2 register writes for the sound (what
  `tools/iopemu.py` and `tools/sound_ee.py` compare). So moving to the
  cores changes no checked behaviour; it is files moving between crates.
- Backends never depend on each other or on the game; only on their core
  (and `piney-data` for assets).
- Shared GPU plumbing both renderers use (the device and surface, the
  headless device, texture decode from CCS) lives in the core, so the two
  backends do not grow two copies of it.
- `piney-game` picks a backend at start-up from the settings and the
  command line (`--renderer classic|modern`, `--sound classic|modern`),
  and the console can switch at run time where the backend allows.
- First step, before any modern work: make the cores and move Classic
  into its crates with no behaviour change (the suite, the harnesses and
  the golden shots identical), so Modern starts from a clean seam.

## Rendering

1. **Measure first.** Frame-time trace on release builds at the slow
   scenes; the perf list in rust-cleanup.md section 5 is the starting
   suspects.
2. **Retained resources.** Today every frame re-sends vertices and
   allocates per draw. Upload each file's meshes, textures and palettes
   once, keyed by stable ids (`FileId`, `ModelId`, `TexId`); the game's
   frame names instances (mesh, transform, material parameters, skin
   matrices, morph weights, UV scroll), not vertices. Persistent and ring
   buffers, cached bind groups and pipelines, no per-frame textures.
3. **The GPU does the per-vertex work.** Skinning, morphs, the VU1
   lighting and the fog move into shaders (the same arithmetic, so the
   faithful look survives); instancing and batching by material and
   state, sorted to cut pipeline switches, with the draw order kept where
   the game's blending depends on it (transparent passes in the game's
   order).
4. **A render graph.** The frame-buffer tricks become passes: the boss
   blur and other feedback, water and heat haze sampling the scene colour,
   the game-over squeeze, fades, the deflicker filter, shadows. Each pass
   names its inputs and outputs; no ad-hoc copies.
5. **Resolution and aspect.** Draw at the window's resolution and any
   aspect ratio: widescreen with the field of view widened, the HUD
   anchored to its corners (the HUD scale already moves that way); MSAA
   or TAA; mipmaps and anisotropic filtering where the game sampled
   nearest; optional texture upscaling.
6. **Frame pacing.** The game ticks at its own 30 or 60 Hz; the renderer
   runs at the display's rate and interpolates transforms, the camera and
   animation between ticks. Game state is never interpolated.
7. **Then the extras**, each optional: real shadow maps for the
   projected ones, better lighting, post effects (bloom, colour grading),
   all off in Classic.

## Sound

1. **No lock in the callback.** The game thread sends commands over a
   lock-free queue; the audio thread owns the mixer. No allocation on the
   audio thread (rust-cleanup.md section 5 has the current ones).
2. **Decode once.** ADPCM samples decoded to PCM when a bank loads, cached;
   streams (BGM.BIN, voices, the movies' sound) decoded on a worker thread
   ahead of playback.
3. **A modern mixer.** A bus graph (music, effects, voices, ambience)
   with its own volumes in the Option menu; high-quality resampling to the
   device's own rate in place of the SPU2's Gaussian table; a limiter on
   the master.
4. **The sequencer stays the game's.** The MIDI sequencer and the
   synthesizer's note, envelope and pitch logic keep the game's timing and
   values (that is what is checked against the IOP modules); only how a
   voice becomes samples changes.
5. **Space.** The game's 3D sound (`se3d`: distance, pan) mapped to proper
   spatial audio: stereo panning laws, surround (5.1 / 7.1) where the
   device has it, optional HRTF on headphones.
6. **Reverb.** A modern algorithmic or convolution reverb tuned per
   SPU2 preset, Classic keeping the SPU2's own.
7. **Latency.** A small device buffer once the lock is gone, so sound
   effects land on their frame.

## Built on defs and systems: achievements

The maintainer's idea, 2026-10-30. Achievements are a def and a system
like any other (plans/rust-cleanup.md, "Defs and systems"):

```rust
pub struct AchievementDef {
    id: &'static str, name: &'static str, desc: &'static str,
    when: Condition,     // over the frame's events and the save: Event(..), Flag(saveData + off), Story(n), Count(..)
    volumes: VolumeSet,
}
trait AchievementBackend { fn unlock(&mut self, id: &str); fn progress(&mut self, id: &str, n: u32, of: u32); }
```

- The `Achievements` system only observes: it reads each frame's events and
  the save and never writes, so the game is bit-exact with it on or off.
- Backends behind one trait, interchangeable:
  - **Local** (default): a file in the port's folder and an in-game list.
  - **RetroAchievements**: RA's PS2 sets are conditions on the console's
    memory addresses, the game named by its disc hash. The port already
    holds the save and many globals at their game addresses, so a virtual
    memory view mapping those addresses onto the port's state could run
    RA's own .hack sets through `rcheevos` (MIT). Needs RA to accept a
    native port as a client: their call, asked before building.
  - **Steam**: `steamworks-rs` (MIT/Apache) over Valve's redistributable
    SDK; the port cannot ship on Steam (the games are not ours), so this
    only serves a non-Steam shortcut. Cheap to add behind the trait.
- The achievement list itself is data: a table of `AchievementDef`s per
  volume, written by us (never copied from another project's set).

## Order

The crate split first (Classic moved into its crates, no behaviour change),
then rendering 1-3 (the largest perf win), sound 1-2 (the glitch risk), then
rendering 4-6 and sound 3-7, then the optional extras. Each step lands
with Classic still working and the contract checks unchanged.
