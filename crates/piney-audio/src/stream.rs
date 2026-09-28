//! The sequenced music around an in-engine stream (`docs/engine/sound.md`,
//! "Streams"): `ccSndStreamCtrl` (0x0017d190), which `ccRequestLoadStream`
//! calls before the stream plays and after it returns, and `ccSndStreamBGM`
//! (0x0017cb20), which a stream's note of event 4 calls through
//! `ccSndStreamSE` (0x0017caa0). Both work on the area's bank with the
//! driver's pieces; their fades run in [`Driver::frame`]. While the desktop's
//! Audio screen plays a movie (`ccSnd +0x62`) the caller does not call them.

use crate::Audio;
use crate::driver::{Command, Driver};
use piney_data::sound::Tables;
use piney_data::volume::Volume;

/// What `ccSndStreamCtrl` reads of the game besides the stream:
/// `game.status` (+0x00; 7 is the stream viewer, where nothing changes
/// after a stream) and `game.field` (+0x24; stream 107, the Chaos Gate,
/// starts the field's music again only in area 16).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StreamGame {
    pub status: i32,
    pub field: i32,
}

/// One record of a `strSndTbl` BGM table (12 bytes; +0x08 is not read).
/// `cmd`: 0 play `sq` at its table volume; 1 stop `sq`, set port `sq + 1`
/// to `vol` unless it is -1, and stop `sq2` unless it is -1; 2 play `sq`
/// from volume 0 and fade it to `vol` 256ths over `time` frames; 3 fade
/// `sq` to `vol` 256ths over `time` frames and stop it; 4 fade `sq` out
/// over `time` frames and `sq2` in from 0. A record with `sq` below 0 is
/// skipped - or, with `cmd` 5, ends the table: every later note reads it
/// again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StrBgm {
    pub sq: i16,
    pub sq2: i16,
    pub time: i16,
    pub vol: u16,
    pub cmd: i16,
}

/// `ccSndStreamCtrl`'s `sd->size` bits: change the music before the stream
/// (bit 0) and after it (bit 1).
pub const BEFORE: i32 = 1;
pub const AFTER: i32 = 2;

/// The fades' length: 30 frames.
const FADE: i32 = 30;

/// `seData` row `ccSndStreamCtrl(13, .., 1)` ends a looping sound effect
/// with.
pub const LOOP_OFF_SE: usize = 42;

/// `ccSqPlayVol(sq, vol)` (0x001799b0): as `ccSqPlay`, at `vol` (no more
/// than the table volume) instead of the table volume.
pub fn sq_play_vol(d: &mut Driver, sq: usize, vol: u16, out: &mut Vec<Command>) {
    if sq >= d.sq_num || matches!(d.sq_status[sq], 1 | 3) {
        return;
    }
    let t = d.sqtbl[sq];
    let v = vol.min(t.vol);
    let hd = t.hd_port as usize;
    d.port_vol[hd] = v;
    d.port_vol_set(hd, v, out);
    out.push(Command::Play(t.midi_port as usize));
    d.sq_status[sq] = 1;
}

/// `ccSndStreamCtrl(num, sd, when)` (INF 0x0017d190, MUT 0x0017f870, OUT
/// 0x0017f2c0, QUA 0x0017f220): `size` is the stream header's `size`, `after`
/// whether the stream has played. Each volume has its own switch over the
/// stream numbers (Quarantine's is Outbreak's), made of a few pieces: out
/// (fade to 0 over 30 frames, playing on or stopped), to half and back, in
/// (`ccSqPlayVol(sq, 0)` and a fade up), the hand-over to the second
/// arrangement, and the battle switch. The cases by volume are in
/// docs/engine/sound.md ("Streams").
#[allow(clippy::too_many_arguments)]
pub fn stream_ctrl(
    d: &mut Driver,
    t: &Tables,
    volume: Volume,
    num: usize,
    size: i32,
    after: bool,
    game: StreamGame,
    out: &mut Vec<Command>,
) {
    let later = volume != Volume::Inf;
    let outbreak = matches!(volume, Volume::Out | Volume::Qua);
    if !after {
        if size & BEFORE == 0 {
            return;
        }
        match (later, num) {
            (false, 112..=119) | (true, 118..=125) => d.sq_fade(0, 128, FADE, 1),
            (_, 25 | 26) => d.sq_fade(0, 0, FADE, 1),
            (false, 57) | (true, 57 | 132) => d.sq_fade(1, 0, FADE, 3),
            (true, 87 | 134) => d.sq_fade(0, 0, FADE, 3),
            (true, 126 | 131) => {
                let sq = if outbreak && d.battle_music { 1 } else { 0 };
                d.sq_fade(sq, 0, FADE, 3);
            }
            (_, 56) => battle_switch(d, out),
            (true, 133) => {
                battle_switch(d, out);
                d.sq_status[1] = 1;
            }
            (false, 8 | 58) | (true, 8 | 58 | 127..=130) => hand_over(d, out),
            _ => {}
        }
        return;
    }
    if size & AFTER == 0 || game.status == 7 {
        return;
    }
    match (later, num) {
        (false, 112..=119) | (true, 118..=125) => d.sq_fade(0, 256, FADE, 1),
        (false, 107) | (true, 113) => {
            if game.field == 16 {
                d.sq_play(0, out);
                if d.sq_num >= 3 {
                    d.sq_play(2, out);
                }
            }
        }
        (_, 3 | 25 | 26) | (true, 134) => fade_in(d, 0, out),
        (true, 87 | 131) => fade_in(d, 1, out),
        (true, 133) => {
            d.battle_bank = true;
            d.sq_stop(1, out);
            fade_in(d, 0, out);
        }
        (true, 132) if outbreak => {
            d.battle_bank = true;
            fade_in(d, 0, out);
        }
        (_, 8) => {
            d.sq_stop(0, out);
            d.sq_stop(1, out);
        }
        (_, 12) => d.sq_play(1, out),
        (_, 13) => {
            d.scene_mode = 0;
            loop_off(t, &mut d.loop_id, out);
        }
        _ => {}
    }
}

/// `ccSqPlayVol(sq, 0)`, then `sq` fades in to its table volume over 30
/// frames.
fn fade_in(d: &mut Driver, sq: usize, out: &mut Vec<Command>) {
    sq_play_vol(d, sq, 0, out);
    d.sq_fade(sq, 256, FADE, 1);
}

/// The battle switch off (`ccSnd +0x5f`), SNDBASE's `bgmChange`, sequence 0
/// out and stopped, sequence 1 in to its table volume.
fn battle_switch(d: &mut Driver, out: &mut Vec<Command>) {
    d.battle_bank = false;
    out.push(Command::BgmChange);
    d.sq_fade(0, 0, FADE, 3);
    d.sq_fade(1, 256, FADE, 1);
}

/// Port 2 at full (256 of `bgmVol`), SNDBASE's `bgmChange` (sequence 1
/// starts where sequence 0 is), `sqStatus[1]` 1, sequence 0 out and
/// stopped: the area's music hands over to its second arrangement.
/// `ccSnd +0x105` is cleared.
fn hand_over(d: &mut Driver, out: &mut Vec<Command>) {
    d.scene_mode = 0;
    d.port_vol[2] = 256;
    d.port_vol_set(2, 256, out);
    out.push(Command::BgmChange);
    d.sq_status[1] = 1;
    d.sq_fade(0, 0, FADE, 3);
}

/// Stream 13's after: the first taken `loopID` slot's sound effect ended
/// with `FD 10 ch note id 00 00` on port 0 (`sceMSIn_PutHsMsg`), `seData[42]`
/// giving the channel and note; the game then frees the slot the id
/// indexes (not the slot it read), which is the same slot when ids are
/// their slots' indices.
fn loop_off(t: &Tables, loops: &mut [i8; 8], out: &mut Vec<Command>) {
    let Some(&id) = loops.iter().find(|&&id| id != -1) else { return };
    let se = &t.se[LOOP_OFF_SE];
    out.push(Command::Msg(vec![0xfd, 0x10, se.ch as u8, se.note as u8, id as u8, 0, 0]));
    if let Some(slot) = usize::try_from(id).ok().and_then(|i| loops.get_mut(i)) {
        *slot = -1;
    }
}

/// `ccSndStreamBGM(param)` (0x0017cb20) on the record the table's cursor
/// is at (`param` is not read); `rec.sq` is 0 or more (the cursor's walk,
/// which skips the others, is the caller's).
pub fn stream_bgm(d: &mut Driver, rec: StrBgm, out: &mut Vec<Command>) {
    let Ok(sq) = usize::try_from(rec.sq) else { return };
    match rec.cmd {
        0 => d.sq_play(sq, out),
        1 => {
            d.sq_stop(sq, out);
            if rec.vol != 0xffff {
                let port = sq + 1;
                if port < 4 {
                    d.port_vol[port] = rec.vol;
                    d.port_vol_set(port, rec.vol, out);
                }
            }
            if let Ok(sq2) = usize::try_from(rec.sq2) {
                d.sq_stop(sq2, out);
            }
        }
        2 => {
            if sq < 3 {
                sq_play_vol(d, sq, 0, out);
            }
            d.sq_fade(sq, rec.vol, i32::from(rec.time), 1);
        }
        3 => d.sq_fade(sq, rec.vol, i32::from(rec.time), 3),
        4 => {
            d.sq_fade(sq, 0, i32::from(rec.time), 3);
            if let Ok(sq2) = usize::try_from(rec.sq2)
                && sq2 < 3
            {
                sq_play_vol(d, sq2, 0, out);
                d.sq_fade(sq2, 256, i32::from(rec.time), 1);
            }
        }
        _ => {}
    }
}

impl Audio {
    /// `ccSndStreamCtrl(num, sd, after)` on the loaded bank: see
    /// [`stream_ctrl`].
    pub fn stream_music(&self, num: usize, size: i32, after: bool, game: StreamGame) {
        let volume = self.snd.volume();
        self.run(|d, t, out| stream_ctrl(d, t, volume, num, size, after, game, out));
    }

    /// `ccSndStreamBGM`: a record of the stream's BGM table (see
    /// [`stream_bgm`]).
    pub fn stream_bgm(&self, rec: StrBgm) {
        self.run(|d, _, out| stream_bgm(d, rec, out));
    }
}
