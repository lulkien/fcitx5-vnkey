# fcitx5-vnkey — Vietnamese input method for fcitx5

A Vietnamese input-method engine in Rust, with an fcitx5 addon around it. The
engine and all key-handling policy are Rust; the only C++ is a small façade that
satisfies fcitx5's C++-only `InputMethodEngine` API.

```
crates/vnkey-core     engine: composition, rules, flattener, phonotactics,
                       charsets, embedded spell-check dictionary
crates/vnkey-addon    preedit/commit policy, macros, cxx bridge,
                       cpp/facade.{h,cpp} (the fcitx5 binding)
```

Status, known deviations and verification steps: [docs/PORT_STATUS.md](docs/PORT_STATUS.md).

```
cargo test                       # 59 tests, no system dependencies
cargo check --features ffi       # bridge; the façade needs fcitx5 headers
sudo apt install libfcitx5core-dev libfcitx5config-dev libfcitx5utils-dev
cmake -B build -DCMAKE_INSTALL_PREFIX=/usr && cmake --build build
sudo cmake --install build
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
- `crates/vnkey-addon` (addon logic + fcitx5 façade) carries LGPL-2.1-or-later
  portions and stays under that license; see `LICENSES/LGPL-2.1-or-later.txt`.

`crates/vnkey-addon/data/vietnamese.cm.dict` is third-party data whose
provenance is not established — check it before relying on a single-license
claim.
