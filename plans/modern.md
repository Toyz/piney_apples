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
  and as an option for players who want the original look and sound.
  (Decision for the maintainer: keep it, or drop it once the modern path
  is trusted.)

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

## Order

Rendering 1-3 (the largest perf win), sound 1-2 (the glitch risk), then
rendering 4-6 and sound 3-7, then the optional extras. Each step lands
with Classic still working and the contract checks unchanged.
