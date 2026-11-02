//! Players' pad logs replayed through the launcher (issues #78-#80): a
//! diagnostic that prints each change of scene, the field's new calls and
//! the slow frames, and shoots a window of frames.

use super::*;

/// `PINEY_PADLOG=FILE [PINEY_LAUNCHER_ROW=3] [PINEY_FROM=F PINEY_TO=G
/// PINEY_EVERY=N] [PINEY_SLOW_MS=M] cargo test --release -p piney-game
/// player_log_replay -- --ignored --nocapture`: shots of frames F..G every
/// N to `$PINEY_SHOTS` (/mnt/data/claude/scratch/bugs/replay), and every
/// step slower than M ms (default 25) printed.
#[test]
#[ignore]
fn player_log_replay() {
    let Some(log) = std::env::var_os("PINEY_PADLOG") else { return };
    let env = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<u32>().ok());
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/bugs/replay".into());
    std::fs::create_dir_all(&dir).unwrap();
    let (from, to, every) = (env("PINEY_FROM").unwrap_or(u32::MAX), env("PINEY_TO").unwrap_or(0), env("PINEY_EVERY"));
    let slow_ms = f64::from(env("PINEY_SLOW_MS").unwrap_or(25));
    let mut gs: Option<piney_gs::Gs> = None;
    let mut last = String::new();
    let mut calls_seen = 0usize;
    let mut then = std::time::Instant::now();
    crate::launcher::replay_pad_log(&log.into_string().unwrap(), |f, mode, _events, _audio, frame| {
        let ms = then.elapsed().as_secs_f64() * 1000.0;
        let title = mode.title();
        if ms > slow_ms {
            println!("frame {f}: slow {ms:.1} ms | {title}");
        }
        let overlay = mode.archive();
        if let Some(s) = mode.session() {
            let archive = s.archive.clone();
            if let Stage::Area(a) = &s.stage {
                let w = a.world();
                let sc = w.scene();
                let now = format!(
                    "area {} field {} dungeon {} floor {} block {} event {}",
                    sc.area,
                    sc.field,
                    sc.dungeon,
                    sc.floor,
                    sc.block,
                    w.world_man().event
                );
                if now != last {
                    println!("frame {f}: {now} | {title}");
                    last = now;
                }
                if a.calls().len() < calls_seen {
                    calls_seen = 0;
                }
                for (_, c) in a.calls().iter().skip(calls_seen) {
                    println!("frame {f}: call {c}");
                }
                calls_seen = a.calls().len();
            }
            let shoot = (from..to).contains(&f) && every.is_none_or(|n| (f - from).is_multiple_of(n.max(1)));
            if shoot {
                let gs = gs.get_or_insert_with(|| piney_gs::Gs::headless(piney_gs::Assets::new(archive)).unwrap());
                gs.set_overlay(overlay);
                gs.render(frame);
                let (w, h) = gs.target_size();
                std::fs::write(format!("{dir}/f{f:05}.png"), piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            }
        }
        then = std::time::Instant::now();
    });
}
