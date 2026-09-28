//! The World's fixed sets: its five servers and its five root towns. The
//! game numbers them (a server 0-4 in `EVENTAREA_INFO +0x10`, a town type in
//! `ccScene`); their data (the servers' symbols, the towns' names) are
//! methods generated in `crate::tables`.

/// A server of The World, by the game's number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Server {
    Delta,
    Theta,
    Lambda,
    Sigma,
    Omega,
}

impl Server {
    pub const ALL: [Server; 5] = [Server::Delta, Server::Theta, Server::Lambda, Server::Sigma, Server::Omega];

    /// The server the game's number names (0 Delta .. 4 Omega).
    pub fn from_index(n: i32) -> Option<Server> {
        Self::ALL.get(usize::try_from(n).ok()?).copied()
    }
}

/// A root town, by the game's town type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Town {
    MacAnu,
    DunLoireag,
    CarminaGadelica,
    FortOuph,
    LiaFail,
}

impl Town {
    pub const ALL: [Town; 5] = [Town::MacAnu, Town::DunLoireag, Town::CarminaGadelica, Town::FortOuph, Town::LiaFail];

    /// The town the game's town type names (0 Mac Anu .. 4 Lia Fail).
    pub fn from_index(n: i32) -> Option<Town> {
        Self::ALL.get(usize::try_from(n).ok()?).copied()
    }
}
