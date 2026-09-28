//! A record of the calls the top page makes into the main executable, for
//! checking its logic against the game's own code run in eemu
//! (`tools/test_toppage_rs.py`): while a trace is on, every `ccAnm`,
//! `ccKanji::Disp`, `ccSprite::MakePacket` and `SendPacket`, `ccScFade`
//! entry and `ccView::SetView` call is recorded, in order, with what the
//! game's side of the check reads at that call. Unlike `piney_desktop`'s
//! `anm::trace`, the animations still play: a forward records its result,
//! which the check hands the game's `_AnimateForward`.

use std::cell::RefCell;

/// One call: the operation, the object's label, its arguments.
pub type Call = (&'static str, &'static str, String);

thread_local! {
    static TRACE: RefCell<Option<Vec<Call>>> = const { RefCell::new(None) };
}

/// Start recording (dropping anything recorded before).
pub fn start() {
    TRACE.with(|t| *t.borrow_mut() = Some(Vec::new()));
}

/// Stop recording and return the calls.
pub fn stop() -> Vec<Call> {
    TRACE.with(|t| t.borrow_mut().take().unwrap_or_default())
}

/// The calls so far, keeping the trace on.
pub fn take() -> Vec<Call> {
    TRACE.with(|t| t.borrow_mut().as_mut().map(std::mem::take).unwrap_or_default())
}

/// Record a call when tracing.
pub fn record(op: &'static str, label: &'static str, arg: impl FnOnce() -> String) {
    TRACE.with(|t| {
        if let Some(v) = t.borrow_mut().as_mut() {
            v.push((op, label, arg()));
        }
    });
}
