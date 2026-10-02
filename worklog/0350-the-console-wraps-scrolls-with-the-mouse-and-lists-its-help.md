---
number: 350
title: "The console wraps, scrolls with the mouse and lists its help in columns; render scale, vsync and an FPS cap"
date: 2026-10-02
area: ui, render, tooling
files: crates/piney-game/src/console.rs, crates/piney-game/src/main.rs, crates/piney-game/src/session/tests/side_events.rs, crates/piney-gs/src/lib.rs, crates/piney-gs/src/convert.rs, crates/piney-gs/src/shadow.rs
---

# 350. The console wraps, scrolls with the mouse and lists its help in columns; render scale, vsync and an FPS cap

None of this is the game's. These are player requests for the port's
window: issues #27, #28 and #29, and the user's asks for a scroll bar and a
fixed `help`.

## The console

- **Wrapping (#27).** An answer wider than the window ran off its edge.
  `Console::wrapped` now breaks each answer into rows at its spaces, and a
  word wider than a row at its letters. A help line's second column wraps
  under itself. The typed line scrolls sideways to keep the cursor in view.
  Page Up / Down count rows as drawn.
- **Help columns.** The description column sat at a fixed 140 pixels.
  `pad_log [FILE|stop]` is wider than that and ran into its own text, and
  `deflicker [on|off]` had one space where the split needs two, so it
  never split. The column now starts past the widest command, up to half
  the window. The window's commands are one list, `APP_HELP`, each line
  split by two spaces.
- **Tab.** It took the first word of every `help` line as a command. At the
  title, where there are none, the line "no console commands here" made
  `no` one. Commands now come only from lines that split. `help` also left
  the recorded path: each F1 press used to write a `help` line into the pad
  log.
- **Scroll bar.** A track on the band's right edge, with a thumb as tall as
  the visible share of the history. The mouse wheel scrolls three rows a
  notch, the thumb drags, and a click on the track pages.

- **History.** Picking a part on the four-part selector (`App::boot`)
  made a new `Console`, so the answers and the lines typed went with the
  scene. It now keeps the console and only swaps the fonts
  (`set_fonts`). The lines typed are also kept between runs, in
  `console_history.txt` in the build's folder (the last 200).

`long_answers_wrap`, `tab_completes_the_commands`,
`the_scroll_bar_scrolls` and `the_history_is_kept` hold these. The by-hand `console_picture` now
draws a long line and enough history for the bar.

## The window

- **`vsync [on|off]` (#29)**, `--no-vsync`: the surface's present mode,
  `AutoVsync` or `AutoNoVsync`, reconfigured on the spot.
- **`fps_cap N` (#29)**, `--fps-cap N`, 0 for none: `about_to_wait`
  holds the next redraw until 1/N s after the last picture
  (`ControlFlow::WaitUntil`). The game's frames keep their own clock
  (`tick`'s vblank count), so the cap and vsync change only how often the
  picture is shown.
- **`render_scale N` (#28)**, `--render-scale N`, 1 to 8:
  `Gs::set_scale`. The frame buffer is N times 512 x 448. The draws keep
  the frame's coordinates, since `convert` maps them to clip space by the
  frame's size and normalizes texel coordinates by the logical size. Five
  things had to follow the scale:
  - the scissor rectangles;
  - the frame-buffer copies, both their regions and their textures, and
    the previous frame;
  - the shadow packet's Z copy and composite, which read frame pixels (the
    uniform's spare slot now carries the scale);
  - the composite's scissor;
  - the half-pixel move that makes the GS's corner rule meet the GPU's
    centre rule (`PIXEL_CENTRE`, now `0.5 / scale`). Without it every
    sprite edge fell one pixel out at scale 2, which the first run of the
    new test caught.

  `a_scaled_frame_buffer_draws_the_frame_bigger` draws a frame at scale 1
  and 2: flat sprites, a scissored one, a frame-buffer copy drawn back and
  a shadow packet. At 2 every pixel is the scale-1 pixel doubled, within 2
  of 255, except along the shadow's bilinear edge, which is checked inside.
  Story 19's start shot at both scales lines up, sharper at 2. The other 22
  GS tests are unchanged at scale 1.

`the_tutorial_statue_s_glow_ends` read DATA.BIN a second time for its GS.
Under the full suite's 8 GB cap, in parallel, that read failed now and
then: a `read_path` unwrap, later a 76 MB allocation abort. It now uses the
session's archive.

**Still unknown:** what the console's Tab report meant (BUGS); the
REGION_REPEAT path reads a frame-buffer copy by its own texel count, so a
repeat-clamped copy at a scale above 1 would mask in physical texels (no
frame drawn so far reads a copy that way).
