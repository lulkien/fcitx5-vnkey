# fcitx5-vnkey — Vietnamese input method for fcitx5

A Vietnamese input-method engine in Rust, with an fcitx5 addon around it. The
engine and all key-handling policy are Rust; the only C++ is a small facade that
satisfies fcitx5's C++-only `InputMethodEngine` API.

```
crates/vnkey-core     engine: composition, rules, flattener, phonotactics,
                       charsets, embedded spell-check dictionary
crates/vnkey-addon    preedit/commit policy, macros, cxx bridge,
                       cpp/facade.{h,cpp} (the fcitx5 binding)
tools/defs/           input-method rules and charset tables (TSV: the data)
tools/gen_tables.py   formats those tables into vnkey-core/src/{im_defs,charset}.rs
```

```
just test                        # 63 tests, no system dependencies
just check                       # factory symbol exported, no unresolved symbols
sudo apt install libfcitx5core-dev libfcitx5config-dev libfcitx5utils-dev
just build
DESTDIR=/tmp/stage just install  # staging; drop DESTDIR and use sudo to install
sudo just install
fcitx5 -r                        # then add "VnKey" in the input methods
```

The addon installs as `vnkey` / `libvnkey.so`, so it can be selected next
to any other Vietnamese input method.

## License

Released into the public domain under the [Unlicense](UNLICENSE), to the extent
the authors hold the rights.

Two exceptions, both required by the licenses of the software this project ports
and both recorded in [NOTICE](NOTICE):

- `crates/vnkey-core` carries MIT-licensed portions; the MIT notice is retained.
- `crates/vnkey-addon` (addon logic + fcitx5 facade) carries LGPL-2.1-or-later
  portions and stays under that license; see `LICENSES/LGPL-2.1-or-later.txt`.

`crates/vnkey-addon/data/vietnamese.cm.dict` is third-party data whose
provenance is not established — check it before relying on a single-license
claim.
