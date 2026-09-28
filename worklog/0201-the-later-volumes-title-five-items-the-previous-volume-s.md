---
number: 201
title: "The later volumes' title: five items, the previous volume's save, every branch of the opening checked on the four volumes"
date: 2026-09-26
area: volumes
files: crates/piney-demo/src/opening.rs, crates/piney-demo/src/names.rs, crates/piney-demo/src/lib.rs, crates/piney-demo/src/dataload.rs, crates/piney-demo/examples/demo_probe.rs, tools/test_demo_rs.py, docs/engine/title.md
---

# 201. The later volumes' title: five items, the previous volume's save, every branch of the opening checked on the four volumes

After 0200 Mutation reached its title, but only the memory-card question
drew. The logo and menu did not. The port's `ccOpening_Control` said it
held volume 1 only.

## Where the volumes differ

Fifteen of the control's functions branch on `m_NowVol` (+0x8). A scan
of the overlay found them:

- the eight `Set*`;
- `ChangeMainAct`, `SwitchCur`;
- `AllAnimate`, `AllTransparency`, `AllDraw`;
- `PlayOpeningStream`, `SetVolCcs`, `Init`.

Infection's own DEMO.PRG holds all four volumes' branches and animation
names. Its DATA.BIN holds `title2`-`title4` too. `voldiff.py ported`
says:

- Mutation's copies of these functions are byte for byte Infection's;
- Outbreak's and Quarantine's are recompiled copies.

So Infection's code for `m_NowVol` 2-4 is the later titles. Running each
`Set*` in eemu for volumes 1-4 and dumping the control's name arrays gave
the names at once (title.md, "Volumes 2-4"):

- five items, the fourth the previous volume's save ("CONVERT");
- the backdrop `ne{n}1`/`ne{n}0`;
- a fifth dummy in each panel;
- items 3 and 4 swapping volume 1's 30 and 40 dummy objects.

The behaviour then came from the branches themselves:

- `SwitchCur`'s neighbour rule over five items;
- `SetNeutral`'s jump to the cursor's own item;
- `ChangeMainAct`'s second table (3 to `SetNextData`);
- `SetParodyGame` on item 4;
- the per-frame loops over items 3 and 4;
- `AllTransparency` leaving item 3 alone while its panel is open;
- `SetNextData` (`SetDataLoad` with a mode argument, clearing
  `m_OptSW`);
- `PlayNextData`, whose flash ends with `m_NextSW` 0, so the window
  closes before it leaves with `m_StartMode` 4;
- `PlayOpeningStream`'s streams 21/22, 50/51 and 75/76.

One piece is Mutation's alone. Infection's `ccThDemo` never raises
`m_Max_CurNO` from `Init`'s 3. Mutation's calls a new function after
`Init` (MUT 0x00419630) that sets 4 on volumes 2-4.
`Opening::set_max_cur` is that call, made where the title starts.
Nothing sets `m_ParoFLG` on any disc, so the fifth item stays hidden.

`NextDataLoad_Control` is `Data_Control` with `m_PrevFlg` 1, set after
its own `Init`. The port keeps it as a second `DataLoad` (`new_next`,
`init_next`). What that flag changes inside `Data_Control` is the next
step.

## Checked

`test_demo_rs.py` now runs these checks once per volume through a
`volumes` decorator:

- set (with `setnext`), switch, cursor, count, transparency, animation,
  draw;
- `Main` (actions 10 and 11 allowed from volume 2);
- `Play*` (with a `playnext` sweep like `playload`'s);
- the boot.

The eemu machine takes the volume in `m_NowVol`. The probe takes a
`vol N` line and loads `title<N>`. All 18 tests pass on the four
volumes, and Infection's results are unchanged. Mutation now shows its
own logo and backdrop, and a four-item menu with the cursor on CONVERT.

**Still unknown:** The previous volume's load screen (`Data_Control`'s
`m_PrevFlg` branches: reading Infection's card from Mutation) and
`ConvGame` are not ported, so CONVERT opens a window that loads nothing
yet. Outbreak's and Quarantine's titles were not run.
