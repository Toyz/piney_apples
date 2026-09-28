//! `battle_probe` requests for the party's bookkeeping task
//! (`piney_battle::frame`), as `tools/test_battle_frame_rs.py` sends them:
//! `spcthread OPN OLD COND STRAT N (AREA INBATTLE PUPPET OPERATION
//! PARTYNUM)*N` runs `ccThSpc` from its start over N frames, the inputs of
//! frame `i` in place before its pass; per frame it prints the battle
//! condition, `spcOpnCnt`, the strategy, `spcCheckInBattleOld` and the
//! operation Kite shouted (-2 for none).

use piney_battle::exp::Party;
use piney_battle::frame::{SAVE_OPERATION, SpcOut, SpcThread};
use piney_battle::tables::Tables;
use piney_data::save::SaveData;

use crate::Toks;

pub(crate) fn handle(cmd: &str, t: &mut Toks, tables: &mut Tables) -> Option<String> {
    match cmd {
        "spcthread" => Some(spc_thread(t, tables)),
        _ => None,
    }
}

fn spc_thread(t: &mut Toks, tables: &Tables) -> String {
    let mut s =
        SpcThread { opn_cnt: t.i16(), in_battle_old: t.i16(), battle_condition: t.i16(), party_strategy: t.i32() };
    let n = t.int() as usize;
    let frames: Vec<[i32; 5]> = (0..n).map(|_| std::array::from_fn(|_| t.i32())).collect();
    let mut save = SaveData::new();
    let mut party = Party { members: [Some(0), Some(1), Some(2)], ids: [0, 1, 2], num: 0 };
    let put = |save: &mut SaveData, party: &mut Party, f: &[i32; 5]| {
        save.set_u8(SAVE_OPERATION, f[3] as u8);
        party.num = f[4];
    };
    put(&mut save, &mut party, &frames[0]);
    s.start(&save, frames[0][1]);
    let mut out = Vec::new();
    for f in &frames {
        put(&mut save, &mut party, f);
        let shout = match s.frame(tables, &mut save, &party, f[0], f[1], f[2] != 0) {
            Some(SpcOut::Shout { operation }) => i32::from(operation),
            None => -2,
        };
        out.push(format!("[{},{},{},{},{}]", s.battle_condition, s.opn_cnt, s.party_strategy, s.in_battle_old, shout));
    }
    format!("{{\"frames\":[{}]}}", out.join(","))
}
