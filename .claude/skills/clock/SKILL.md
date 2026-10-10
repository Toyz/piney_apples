---
name: clock
description: Start the worklog clock when beginning any piece of work on piney_apples that will end in a worklog entry - a bug to diagnose, a subsystem to build, a question to answer, a format to decode. Use as soon as the task is understood, before investigating or editing anything. Also use when the user says "start the clock", "time this", "/clock".
---

# Clock

The worklog records how long each piece of work took. The tool keeps the time,
not you: you have no sense of elapsed time, and a number you estimate would be
confident and wrong. So the one thing to do is start the clock when the work
starts.

```sh
cairns start "what you are about to find out"
```

Then work. When it is done, the `worklog` skill writes the entry, and
`cairns new` records how long the clock ran as the entry's `took:` and stops
it. Nothing else to remember.

## When

At the beginning of anything that will end in an entry - before reading code,
before running anything. Starting late undercounts; starting at all is what
matters most.

Not for trivial edits, formatting, or questions answered in a sentence: those
get no entry, so they need no clock.

## When things change

```sh
cairns clock            # what is running, and for how long
cairns start "..."      # a new piece of work: replaces the running clock, and says so
cairns start --cancel   # the work was abandoned and gets no entry
```

If a clock is already running when a task begins, it belongs to earlier work
that never got its entry. Write that entry first if the work was worth one;
otherwise `cairns start` replaces it.

If the work spanned a long break, the clock includes the break. Say so when
writing the entry: `cairns new --took 45m` records the time spent instead.

<!-- cairns:project -->

## This project

<Anything specific to this repo about what is timed. Written by hand -
`cairns init` preserves everything below the marker above.>
