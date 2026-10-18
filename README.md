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
debian/               packaging: hand-written rules, no debhelper
```

## Build

Needs a Rust toolchain (1.82+), a C++ compiler, `pkg-config`, `just`, and the
fcitx5 development headers (`python3` only for regenerating the tables,
`dpkg-dev` only for the Debian package):

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
sudo just --no-deps install   # as root: copies 21 files into /usr
fcitx5 -r                     # restart the daemon
```

`--no-deps` matters: without it, `just install` re-runs the build under sudo and
leaves `target/release/libvnkey.so` and `out/*.conf` owned by root, which makes
the next user-side `just build` fail on the link step.

To inspect what would be installed, or to build a package, stage it instead:

```
DESTDIR=/tmp/stage just --no-deps install
find /tmp/stage -type f        # 21 files: lib, 2 descriptors, 2 SVGs, 16 PNG sizes
```

Installing the Debian package below does the same thing, without `just` on the
target machine.

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

## Debian package

`debian/` is hand-written — no debhelper. `debian/rules` calls the same
`just build` / `just install` path the sections above document, then `dpkg-deb`
packs the staged tree, so a package contains exactly the 21 files `just install`
writes — verified by extracting the `.deb` and diffing it against a `DESTDIR`
stage.

```
just deb                          # -> ../fcitx5-vnkey_<version>-1_amd64.deb
dpkg-buildpackage -b -us -uc -d   # the same thing, spelled out
sudo apt install ./fcitx5-vnkey_*.deb
```

The `-d` skips `dpkg-checkbuilddeps`: a rustup toolchain is not a Debian
package, so the declared Build-Depends cannot be satisfied that way even with
`cargo` on `PATH`. Drop the flag on a machine with distro `cargo`/`rustc`.
`debian/rules` refuses to build when the `Cargo.toml` version and
`debian/changelog` disagree.

## Releases

`just bump` is the whole release step — it writes the version into `Cargo.toml`,
`Cargo.lock` and `debian/changelog`, commits, and creates the tag:

```
just bump 0.2.0          # -> commit "release: v0.2.0" + annotated tag v0.2.0
git push --follow-tags   # the tag is what starts the release build
```

Pushing a `v*` tag runs `.github/workflows/release.yml`, which checks that the
tag, `Cargo.toml` and `debian/changelog` all carry the same version, runs
`just check` and `just test`, builds the `.deb` with `just deb`, and publishes a
GitHub release containing the package and a `SHA256SUMS` file. A tag that does
not match the sources fails the build before anything is published — that is
what catches a tag made without `just bump`.

`just bump` refuses to run on a dirty tree, on a version that is already
current, or when the tag already exists; it does not push.

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
