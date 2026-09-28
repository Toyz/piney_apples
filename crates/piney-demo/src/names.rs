//! The names DEMO.PRG's code gives its animations, objects and files, for
//! Infection (`volumeNum` 1): string literals of the overlay the code stores
//! into `ccOpening_Control`'s name arrays (`m_ico_D` +0x2c .. `m_window`
//! +0xa4) before each `ccAnm::SetAnm`; every animation is in `title1`. Slots
//! are indexed by the menu item: 0 New Game, 1 Load, 2 Option, 3 Parody
//! (shown only while `m_ParoFLG` is set, which Infection never does). `None`
//! is a null pointer in the array.

/// `SetVolCcs` (0x00405e80): the title's scene file for volume 1 (`title1`
/// .. `title4` by volume), also `DemoFileList[0]` (0x0040dc40,
/// `TITLE1.CCS`).
pub const TITLE_FILE: &str = "title1";

/// `SetVolCcs`'s scene file for a volume: `title1` .. `title4` by
/// `volumeNum`.
pub fn title_file(volume: piney_data::volume::Volume) -> &'static str {
    ["title1", "title2", "title3", "title4"][(volume.number() - 1) as usize]
}
/// `DemoFileWINDOWList` (0x0040dc70): the message window's file, loaded
/// with the title for the memory-card dialogs.
pub const WINDOW_FILE: &str = "xwindo10";
/// The overlay (`ccSetupDemo` loads `cdrom0:\DATA\DEMO.PRG`).
pub const PRG_PATH: &str = "DATA/DEMO.PRG";
/// The overlay's name in its header.
pub const PRG_NAME: &str = "demo.prg";

/// `SetStream` (0x00407670): the camera (`m_Camera`).
pub const CAMERA: &str = "ANM_xdtcam00";
/// `Init` (0x00405f50): `m_Cur`'s texture, a `ccMask(10, 0)` on the title's
/// layer that nothing draws.
pub const CURSOR_TEXTURE: &str = "TEX_xgtcur00";

/// `m_back[0..4]` as `SetBootMemCard` (0x004076f0) and `SetNeutral`
/// (0x00407950) set them for volume 1: the title logo, the backdrop, the
/// digits and copyright behind, and the screen-out (`A_BACK[0..4]`).
pub const BACK: [&str; 4] = ["ANM_xdt_ne01", "ANM_xdt_ne00", "ANM_xdt_ne09", "ANM_xdt_ou00"];
/// `SetBootMemCard`'s `m_A_BOOT`: the screen-out again, squashed to half
/// height.
pub const BOOT: &str = "ANM_xdt_ou00";
/// `SetBootMemCard`'s scale for `m_A_BOOT` (`SetMatrix_PosRotXYZScale`).
pub const BOOT_SCALE: [f32; 3] = [1.0, 0.5, 1.0];
/// `SetBootMemCard` steps `A_BACK[3]` and `m_A_BOOT` this many times at once.
pub const BOOT_PRESTEP: u32 = 30;

/// `SetNeutral`: `m_menu`, each item at rest.
pub const MENU: [Option<&str>; 5] =
    [Some("ANM_xdt_ne02"), Some("ANM_xdt_ne03"), Some("ANM_xdt_ne04"), Some("ANM_xdt_ne05"), None];
/// `SetNeutral`: `m_menu_Sel`, the item under the cursor.
pub const MENU_SEL: [Option<&str>; 5] =
    [Some("ANM_xdt_ch00"), Some("ANM_xdt_ch01"), Some("ANM_xdt_ch02"), Some("ANM_xdt_ch03"), None];
/// `SetNeutral`: `m_ico`, the item's turning icon.
pub const ICO: [Option<&str>; 5] =
    [Some("ANM_xdt_ic00"), Some("ANM_xdt_ic01"), Some("ANM_xdt_ic02"), Some("ANM_xdt_ic03"), None];
/// `SetNeutral`: `m_ico_D`, the dummies that place the icons at rest.
pub const ICO_D_NEUTRAL: [Option<&str>; 5] =
    [Some("ANM_xdt_dn00"), Some("ANM_xdt_dn01"), Some("ANM_xdt_dn02"), Some("ANM_xdt_dn03"), None];
/// The dummy objects `GetSubstAdrsF` finds in the `m_ico_D` animations.
pub const DAM_ICO: [&str; 5] =
    ["OBJ_dam_ico_00_", "OBJ_dam_ico_10_", "OBJ_dam_ico_20_", "OBJ_dam_ico_30_", "OBJ_dam_ico_40_"];
/// The dummy objects in the `m_menu_D` animations.
pub const DAM_MEN: [&str; 5] =
    ["OBJ_dam_men_00_", "OBJ_dam_men_10_", "OBJ_dam_men_20_", "OBJ_dam_men_30_", "OBJ_dam_men_40_"];

/// `SetNewGame` (0x00408240): `A_APP[0]`'s decide animation.
pub const NEW_GAME_DECIDE: &str = "ANM_xdt_de00";
/// `SetParodyGame` (0x00409d40): `A_APP[3]`'s.
pub const PARODY_DECIDE: &str = "ANM_xdt_de03";

/// The dummies and window of a panel (Load or Option) opening or closing:
/// `m_ico_D`, `m_menu_D`, `m_window`, `m_back[0]` and `m_back[3]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Panel {
    pub ico_d: [Option<&'static str>; 5],
    pub menu_d: [Option<&'static str>; 5],
    pub window: &'static str,
    pub back0: &'static str,
    pub back3: &'static str,
}

/// The dummies that move the icons and items aside for a panel.
const ICO_D_OPEN: [Option<&str>; 5] =
    [Some("ANM_xdt_do08"), Some("ANM_xdt_do00"), Some("ANM_xdt_do01"), Some("ANM_xdt_do02"), None];
const MENU_D_OPEN: [Option<&str>; 5] =
    [Some("ANM_xdt_do09"), Some("ANM_xdt_do04"), Some("ANM_xdt_do05"), Some("ANM_xdt_do06"), None];
const ICO_D_CLOSE: [Option<&str>; 5] =
    [Some("ANM_xdt_dc08"), Some("ANM_xdt_dc00"), Some("ANM_xdt_dc01"), Some("ANM_xdt_dc02"), None];
const MENU_D_CLOSE: [Option<&str>; 5] =
    [Some("ANM_xdt_dc09"), Some("ANM_xdt_dc04"), Some("ANM_xdt_dc05"), Some("ANM_xdt_dc06"), None];

/// `SetDataLoad` (0x004082c0) with `m_DatAct` 0: the load window opens.
pub const LOAD_OPEN: Panel = Panel {
    ico_d: ICO_D_OPEN,
    menu_d: MENU_D_OPEN,
    window: "ANM_xdt_op01",
    back0: "ANM_xdt_op00",
    back3: "ANM_xdt_op03",
};
/// `SetDataLoad` with `m_DatAct` 1: it closes.
pub const LOAD_CLOSE: Panel = Panel {
    ico_d: ICO_D_CLOSE,
    menu_d: MENU_D_CLOSE,
    window: "ANM_xdt_cl01",
    back0: "ANM_xdt_cl00",
    back3: "ANM_xdt_cl03",
};
/// `SetOption(0)` (0x00408ba0): the option window opens.
pub const OPTION_OPEN: Panel = Panel {
    ico_d: ICO_D_OPEN,
    menu_d: MENU_D_OPEN,
    window: "ANM_xdt_op02",
    back0: "ANM_xdt_op00",
    back3: "ANM_xdt_op03",
};
/// `SetOption(1)`: it closes.
pub const OPTION_CLOSE: Panel = Panel {
    ico_d: ICO_D_CLOSE,
    menu_d: MENU_D_CLOSE,
    window: "ANM_xdt_cl02",
    back0: "ANM_xdt_cl00",
    back3: "ANM_xdt_cl03",
};
/// `SetDataLoad`'s `m_menu`: the items while the load window is up (Load
/// decided).
pub const MENU_LOAD: [Option<&str>; 5] =
    [Some("ANM_xdt_de05"), Some("ANM_xdt_de01"), Some("ANM_xdt_de12"), Some("ANM_xdt_de06"), None];
/// `SetOption`'s `m_menu` (Option decided).
pub const MENU_OPTION: [Option<&str>; 5] =
    [Some("ANM_xdt_de05"), Some("ANM_xdt_de11"), Some("ANM_xdt_de02"), Some("ANM_xdt_de06"), None];

/// The volumes 2-4 menu (`m_NowVol` 2-4 in the `Set*` functions; the
/// names are in Infection's overlay too): five items, New Game, Load,
/// Option, the previous volume's save (`SetNextData`) and Parody.
pub mod later {
    use super::Panel;

    pub const MENU: [Option<&str>; 5] =
        [Some("ANM_xdt_ne02"), Some("ANM_xdt_ne03"), Some("ANM_xdt_ne04"), Some("ANM_xdt_ne06"), Some("ANM_xdt_ne15")];
    pub const MENU_SEL: [Option<&str>; 5] =
        [Some("ANM_xdt_ch00"), Some("ANM_xdt_ch01"), Some("ANM_xdt_ch02"), Some("ANM_xdt_ch04"), Some("ANM_xdt_ch13")];
    pub const ICO: [Option<&str>; 5] =
        [Some("ANM_xdt_ic00"), Some("ANM_xdt_ic01"), Some("ANM_xdt_ic02"), Some("ANM_xdt_ic04"), Some("ANM_xdt_ic03")];
    pub const ICO_D_NEUTRAL: [Option<&str>; 5] =
        [Some("ANM_xdt_dn00"), Some("ANM_xdt_dn01"), Some("ANM_xdt_dn02"), Some("ANM_xdt_dn04"), Some("ANM_xdt_dn13")];
    const ICO_D_OPEN: [Option<&str>; 5] =
        [Some("ANM_xdt_do08"), Some("ANM_xdt_do00"), Some("ANM_xdt_do01"), Some("ANM_xdt_do03"), Some("ANM_xdt_do12")];
    const MENU_D_OPEN: [Option<&str>; 5] =
        [Some("ANM_xdt_do09"), Some("ANM_xdt_do04"), Some("ANM_xdt_do05"), Some("ANM_xdt_do07"), Some("ANM_xdt_do16")];
    const ICO_D_CLOSE: [Option<&str>; 5] =
        [Some("ANM_xdt_dc08"), Some("ANM_xdt_dc00"), Some("ANM_xdt_dc01"), Some("ANM_xdt_dc03"), Some("ANM_xdt_dc12")];
    const MENU_D_CLOSE: [Option<&str>; 5] =
        [Some("ANM_xdt_dc09"), Some("ANM_xdt_dc04"), Some("ANM_xdt_dc05"), Some("ANM_xdt_dc07"), Some("ANM_xdt_dc16")];
    pub const MENU_LOAD: [Option<&str>; 5] =
        [Some("ANM_xdt_de05"), Some("ANM_xdt_de01"), Some("ANM_xdt_de12"), Some("ANM_xdt_de14"), Some("ANM_xdt_de06")];
    pub const MENU_OPTION: [Option<&str>; 5] =
        [Some("ANM_xdt_de05"), Some("ANM_xdt_de11"), Some("ANM_xdt_de02"), Some("ANM_xdt_de14"), Some("ANM_xdt_de06")];
    pub const MENU_NEXT: [Option<&str>; 5] =
        [Some("ANM_xdt_de05"), Some("ANM_xdt_de11"), Some("ANM_xdt_de12"), Some("ANM_xdt_de04"), Some("ANM_xdt_de06")];
    /// `SetParodyGame`: `A_APP[4]`'s decide animation.
    pub const PARODY_DECIDE: &str = "ANM_xdt_de13";
    /// The dummies' objects: items 3 and 4 swap volume 1's 30 and 40 (the
    /// previous volume's save takes the Parody item's old place).
    pub const DAM_ICO: [&str; 5] =
        ["OBJ_dam_ico_00_", "OBJ_dam_ico_10_", "OBJ_dam_ico_20_", "OBJ_dam_ico_40_", "OBJ_dam_ico_30_"];
    pub const DAM_MEN: [&str; 5] =
        ["OBJ_dam_men_00_", "OBJ_dam_men_10_", "OBJ_dam_men_20_", "OBJ_dam_men_40_", "OBJ_dam_men_30_"];

    /// A panel of volume `v`: the dummies above, the window, and the
    /// volume's backdrop (`op10`, `op20`, `op30` opening; `cl10` ...
    /// closing).
    pub fn panel(v: i16, open: bool, window: &'static str) -> Panel {
        let (ico_d, menu_d) = if open { (ICO_D_OPEN, MENU_D_OPEN) } else { (ICO_D_CLOSE, MENU_D_CLOSE) };
        let back0 = match (open, v) {
            (true, 2) => "ANM_xdt_op10",
            (true, 3) => "ANM_xdt_op20",
            (true, _) => "ANM_xdt_op30",
            (false, 2) => "ANM_xdt_cl10",
            (false, 3) => "ANM_xdt_cl20",
            (false, _) => "ANM_xdt_cl30",
        };
        let back3 = if open { "ANM_xdt_op03" } else { "ANM_xdt_cl03" };
        Panel { ico_d, menu_d, window, back0, back3 }
    }
}

/// `m_back[0..4]` for a volume: its logo and backdrop (`ne01`/`ne00`,
/// `ne11`/`ne10`, `ne21`/`ne20`, `ne31`/`ne30`), the digits and the
/// screen-out.
pub fn back(v: i16) -> [&'static str; 4] {
    match v {
        2 => ["ANM_xdt_ne11", "ANM_xdt_ne10", "ANM_xdt_ne09", "ANM_xdt_ou00"],
        3 => ["ANM_xdt_ne21", "ANM_xdt_ne20", "ANM_xdt_ne09", "ANM_xdt_ou00"],
        4 => ["ANM_xdt_ne31", "ANM_xdt_ne30", "ANM_xdt_ne09", "ANM_xdt_ou00"],
        _ => BACK,
    }
}

/// `SetNextData` on volume 1 (never reached: its menu has no such item).
pub const MENU_NEXT: [Option<&str>; 5] =
    [Some("ANM_xdt_de05"), Some("ANM_xdt_de11"), Some("ANM_xdt_de12"), Some("ANM_xdt_de06"), None];

/// `PlayOpeningStream`'s stream and its last frame by volume, plain and
/// with `m_ParoFLG`.
pub fn opening_stream(v: i16, parody: bool) -> (i32, u32) {
    match (v, parody) {
        (2, false) => (21, 450),
        (2, true) => (22, 490),
        (3, false) => (50, 450),
        (3, true) => (51, 490),
        (4, false) => (75, 450),
        (4, true) => (76, 490),
        (_, false) => OPENING_STREAM,
        (_, true) => OPENING_STREAM_PARODY,
    }
}

/// The logo movies `LogoMain` (0x00404540) plays by `m_LogoAct`, and the
/// opening movie, on the disc under `PSS/` (`strFileOpen` prefixes
/// `\PSS\`). The flag is `ccDecodeMpeg`'s second argument: play the audio.
pub const LOGO_MOVIES: [(&str, bool); 4] =
    [("PSS/LOGO_B.PSS", false), ("PSS/LOGO_C.PSS", false), ("PSS/LOGO_H.PSS", false), ("PSS/OPENING.PSS", true)];

/// The logo movies for a volume: the opening movie is each disc's own
/// (`opening.pss`, `opening2.pss`, `opening3.pss`, `opening4.pss` in its
/// DEMO.PRG).
pub fn logo_movie(v: i16, act: usize) -> Option<(&'static str, bool)> {
    let (path, audio) = *LOGO_MOVIES.get(act)?;
    let path = match (act, v) {
        (3, 2) => "PSS/OPENING2.PSS",
        (3, 3) => "PSS/OPENING3.PSS",
        (3, 4) => "PSS/OPENING4.PSS",
        _ => path,
    };
    Some((path, audio))
}

/// `PlayOpeningStream` (0x004066b0) for volume 1: the stream number
/// (`ccThExecuteStream`'s parameter, `ccRequestLoadStream(n)`: the
/// `STREAMDATA` list `streamTbl[n]`, `title1` then `title1_st1`, or
/// `title1_st2` for 1) and the frame `PlayOpeningStream` counts it to,
/// plain and with `m_ParoFLG`.
pub const OPENING_STREAM: (i32, u32) = (0, 410);
pub const OPENING_STREAM_PARODY: (i32, u32) = (1, 450);
