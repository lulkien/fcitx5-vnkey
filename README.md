# fcitx5-vnkey — Vietnamese input method for fcitx5

A Vietnamese input-method engine in Rust, with an fcitx5 addon around it. The
engine and all key-handling policy are Rust; the only C++ is a small facade that
satisfies fcitx5's C++-only `InputMethodEngine` API.

Six keyboard layouts are supported: Telex, Telex 2, Telex W, VNI, VIQR and the
Microsoft positional layout.

```
crates/vnkey-core     engine: composition, rules, flattener, phonotactics,
                       charsets, embedded spell-check dictionary
crates/vnkey-addon    preedit/commit policy, macros, cxx bridge,
                       cpp/facade.{h,cpp} (the fcitx5 binding)
tools/defs/           input-method rules and charset tables (TSV: the data)
tools/gen_tables.py   formats those tables into vnkey-core/src/{im_defs,charset}.rs
```

## Build

Needs a Rust toolchain (1.82+), a C++ compiler, `pkg-config`, `just`, and the
fcitx5 development headers:

```
sudo apt install libfcitx5core-dev libfcitx5config-dev libfcitx5utils-dev
just build          # -> target/release/libvnkey.so
just test           # 63 tests, no system dependencies
just check          # tables up to date, factory symbol exported, no undefined symbols
```

The build is a justfile rather than a C++ build system: cargo compiles the
engine, the addon logic, the cxx bridge and `cpp/facade.cpp`, and `just` links
the shared object against the fcitx5 libraries reported by `pkg-config`.

## Install

fcitx5 loads addons only from the library directory it was compiled with, so
there is no per-user install — copying the files into `/usr` requires root:

```
just build                    # as yourself
sudo just --no-deps install   # as root: copies 12 files into /usr
fcitx5 -r                     # restart the daemon
```

`--no-deps` matters: without it, `just install` re-runs the build under sudo and
leaves `target/release/libvnkey.so` and `out/*.conf` owned by root, which makes
the next user-side `just build` fail on the link step.

To inspect what would be installed, or to build a package, stage it instead:

```
DESTDIR=/tmp/stage just --no-deps install
find /tmp/stage -type f        # lib, 2 descriptors, SVG, 8 PNG sizes
```

After restarting fcitx5, add **VnKey** in the input-method list. Restart
without the files installed and the journal reports `Could not locate library
libvnkey.so`, which means the libdir and fcitx5's compiled-in addon directory
disagree.

```
sudo just --no-deps uninstall  # removes exactly the installed files
```

## Vietnamese / English mode

The addon has one daemon-wide mode switch, so a toggle affects every window at
once — no second input-method group needed:

- **Ctrl+Shift+space** (configurable as *Switch Vietnamese/English mode*) flips it.
- The status area shows a checkable **Vietnamese** / **English** item, and the
  input-method icon follows the mode: green **V** for Vietnamese, blue **E** for
  English.
- Switching commits whatever word is being typed instead of dropping it, in
  every open input context.
- In English mode every key passes straight to the application.
- The mode is stored in `conf/vnkey.conf` (`VietnameseMode`), so it survives a
  daemon restart and a config reload.

## Data tables

The input-method rules and the 16 legacy output charsets live as TSV under
`tools/defs/`; the Rust modules `vnkey-core/src/{im_defs,charset}.rs` are
generated from them and are never edited by hand.

```
just tables        # regenerate the .rs files from tools/defs/*.tsv
just tables-check  # fail if the generated files have drifted (part of `just check`)
```

Regeneration needs nothing but Python 3 — no other language toolchain and no
external checkout.

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
