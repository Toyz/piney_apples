#!/usr/bin/env python3
"""The port's data for a volume as the Python tools read it: the files
piney-gen writes into work/data/<volume>/TABLES/ (the same files the build
keeps under PINEY/TABLES/), in piney_data::store's format - numbers
little-endian, text and slices a u32 count then the items, fixed arrays
their items, Option a u8 tag then the value, a struct its fields in order
(plans/build-data.md). Make them with `cargo run --release -p piney-gen --
gen`.

    sound(volume)   the sound tables (piney_data::sound::Tables) as a dict
    setbl(volume)   setbl.cpp's rows and tables (piney_audio::setbl)
"""
import pathlib
import struct

ROOT = pathlib.Path(__file__).resolve().parent.parent
DIRS = {"INF": "infection", "MUT": "mutation", "OUT": "outbreak", "QUA": "quarantine"}


class Reader:
    def __init__(self, data):
        self.b, self.at = data, 0

    def take(self, fmt):
        vals = struct.unpack_from("<" + fmt, self.b, self.at)
        self.at += struct.calcsize("<" + fmt)
        return vals

    def u32(self):
        return self.take("I")[0]

    def count(self):
        return self.u32()

    def slice(self, item):
        return [item() for _ in range(self.count())]

    def opt(self, item):
        return item() if self.take("B")[0] else None


def _file(volume, name):
    path = ROOT / "work" / "data" / DIRS[volume.upper()] / "TABLES" / f"{name}.bin"
    if not path.exists():
        raise SystemExit(f"{path}: run `cargo run --release -p piney-gen -- gen`")
    return path.read_bytes()


CONTEXTS = ("dungeon", "town", "title", "desktop", "toppage", "event", "stream")


def sound(volume="INF"):
    """{"commse": [3], "field": [12 contexts], "dungeon".."stream": context,
    "se": [...], "wave": [(category, fieldType, bgNum, NO)], "bgm": [...]},
    a context {"load": [(ofs, hd, sq1, sq2, sq3, bd)], "vol": [[(midi, hd,
    vol)] * 3], "play": [s8]}."""
    r = Reader(_file(volume, "sound"))

    def context():
        return {
            "load": r.slice(lambda: r.take("6i")),
            "vol": r.slice(lambda: [r.take("iiH") for _ in range(3)]),
            "play": r.slice(lambda: r.take("b")[0]),
        }

    t = {"commse": list(r.take("3I")), "field": [context() for _ in range(12)]}
    for name in CONTEXTS:
        t[name] = context()
    t["se"] = r.slice(lambda: r.take("6bh"))
    t["wave"] = r.slice(lambda: r.take("4i"))
    t["bgm"] = r.slice(lambda: r.take("iiiHH"))
    return t


def setbl(volume="INF"):
    """(base, [(code, note, velocity)], spcSeTbl rows, enemySeTbl rows,
    inuSeData's row); a table's row is None for a NULL pointer."""
    r = Reader(_file(volume, "setbl"))
    base = r.u32()
    rows = r.slice(lambda: r.take("ibb"))
    spc = r.slice(lambda: r.opt(lambda: r.take("H")[0]))
    enemy = r.slice(lambda: r.opt(lambda: r.take("H")[0]))
    inu = r.take("H")[0]
    return base, rows, spc, enemy, inu
