//! Issue #73 on Outbreak: Gorre's fight and the ending after it, as the
//! player's own pad log plays them (`PINEY_PADLOG`).

use piney_battle::boss::Class;

use super::*;

/// The player's log replayed through the launcher: from Gorre's arena on,
/// how many fight frames draw both brothers, then every music event and,
/// every 120 frames, the sequencers and the BGM stream. `PINEY_TITLES`
/// prints each frame's title by log line (the log's own titles run two
/// lines late). A diagnostic: `PINEY_PADLOG=FILE cargo test --release -p
/// piney-game issue_73_replay -- --ignored --nocapture`.
#[test]
#[ignore]
fn issue_73_replay() {
    let Some(log) = std::env::var_os("PINEY_PADLOG") else { return };
    let (mut fight, mut drawn, mut after) = (0u32, 0u32, None::<u32>);
    let mut buf = vec![0i16; 2 * 1600];
    crate::launcher::replay_pad_log(&log.into_string().unwrap(), |f, mode, events, audio, _| {
        audio.render(&mut buf);
        let title = mode.title();
        if std::env::var_os("PINEY_TITLES").is_some() {
            println!("title {f} {title}");
        }
        if let Some(Stage::Area(a)) = mode.session().map(|s| &mut s.stage) {
            let c = a.world_mut().combat_mut();
            if let Some(me) = c.boss_char()
                && let Some(Class::Gorre(x)) =
                    c.scene.chars[me].foe_state().and_then(|f| f.boss.as_ref()).map(|b| &b.class)
            {
                fight += 1;
                drawn += u32::from(x.brother_me.iter().all(|k| c.cast.actors.get(k).is_some_and(|a| a.drawn)));
                after = None;
            } else if fight > 0 && after.is_none() {
                after = Some(f);
                println!("frame {f}: Gorre gone after {fight} frames, both brothers drawn in {drawn}");
            }
        }
        let Some(from) = after else { return };
        for e in events {
            if matches!(
                e,
                Event::DesktopBgm(_)
                    | Event::Bgm(_)
                    | Event::BgmStream(_)
                    | Event::BgmStreamStop
                    | Event::SqLoad(_)
                    | Event::SqPlay(_)
                    | Event::SqStop(_)
                    | Event::SqFade { .. }
                    | Event::BgmCtrl(_)
                    | Event::HoldBgm
                    | Event::AllSoundOff
                    | Event::SoundFadeOut
                    | Event::Movie(_)
                    | Event::StreamBgm(_)
            ) {
                println!("frame {f}: {e:?} | {title}");
            }
        }
        if (f - from).is_multiple_of(120) {
            let seq = audio.with_engine(|e| e.seq.each_ref().map(|q| q.playing()));
            println!("frame {f}: sequences {seq:?}, bgm stream {} | {title}", audio.bgm_streaming());
        }
    });
    println!("{fight} fight frames, both brothers drawn in {drawn}");
}
