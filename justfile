# fcitx5-vnkey build/install tasks.
#
# Everything heavy is cargo: it compiles the engine, the addon logic, the cxx
# bridge and cpp/facade.cpp (via build.rs). This file only does the four things
# a C++ build system would otherwise do: link the shared object, fill in the two
# descriptors, copy the files, and know where they go.

version := `sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1`
prefix := `pkg-config --variable=prefix Fcitx5Core`
libdir := `pkg-config --variable=libdir Fcitx5Core`
# Fcitx5Core.pc defines prefix/libdir but not datadir; FHS says prefix/share.
datadir := prefix / "share"

# pkg-config gives us the flags; the link needs a C++ driver, not cc.
fcitx_cflags := `pkg-config --cflags Fcitx5Core Fcitx5Config Fcitx5Utils`
fcitx_libs := `pkg-config --libs Fcitx5Core Fcitx5Config Fcitx5Utils`

release_dir := "target/release"
out_dir := "out"
static_lib := release_dir / "libvnkey_addon.a"
shared_lib := release_dir / "libvnkey.so"
destdir := env("DESTDIR", "")

icon_sizes := "16 22 24 32 48 64 128 256"

default: build

# Compile everything and link libvnkey.so.
build:
    cargo build --release --features ffi
    c++ -shared -o {{shared_lib}} \
        -Wl,--whole-archive {{static_lib}} -Wl,--no-whole-archive \
        {{fcitx_libs}} \
        -Wl,--no-undefined -Wl,-rpath,'$ORIGIN' \
        -Wl,-soname,libvnkey.so -s
    @mkdir -p {{out_dir}}
    sed 's/@PROJECT_VERSION@/{{version}}/' data/vnkey-addon.conf.in > {{out_dir}}/vnkey-addon.conf
    sed 's/@PROJECT_VERSION@/{{version}}/' data/vnkey.conf.in > {{out_dir}}/vnkey.conf
    @echo "built {{shared_lib}} ({{version}})"

# Install the built files. Needs root for the copies, so build as yourself first:
#   just build && sudo just --no-deps install
# Running the build under sudo would leave root-owned files in target/ and out/.
# DESTDIR stages instead of installing: DESTDIR=/tmp/stage just install
install: build
    install -Dm755 {{shared_lib}} {{destdir}}{{libdir}}/fcitx5/libvnkey.so
    install -Dm644 {{out_dir}}/vnkey-addon.conf {{destdir}}{{datadir}}/fcitx5/addon/vnkey.conf
    install -Dm644 {{out_dir}}/vnkey.conf {{destdir}}{{datadir}}/fcitx5/inputmethod/vnkey.conf
    install -Dm644 data/fcitx_vnkey.svg {{destdir}}{{datadir}}/icons/hicolor/scalable/apps/fcitx_vnkey.svg
    for size in {{icon_sizes}}; do \
        install -Dm644 "data/icons/$size/fcitx_vnkey.png" \
            "{{destdir}}{{datadir}}/icons/hicolor/${size}x${size}/apps/fcitx_vnkey.png"; \
    done
    @echo "installed to {{destdir}}{{prefix}}"

# Remove installed files (same paths as above).
uninstall:
    rm -f {{destdir}}{{libdir}}/fcitx5/libvnkey.so
    rm -f {{destdir}}{{datadir}}/fcitx5/addon/vnkey.conf
    rm -f {{destdir}}{{datadir}}/fcitx5/inputmethod/vnkey.conf
    rm -f {{destdir}}{{datadir}}/icons/hicolor/scalable/apps/fcitx_vnkey.svg
    for size in {{icon_sizes}}; do \
        rm -f "{{destdir}}{{datadir}}/icons/hicolor/${size}x${size}/apps/fcitx_vnkey.png"; \
    done

# Unit tests.
test:
    cargo test

# Lints, including the ffi feature so the bridge is covered.
lint:
    cargo clippy --all-targets --features ffi -- -D warnings

format:
    cargo fmt

# Regenerate the Rust data tables from tools/defs/*.tsv.
tables:
    python3 tools/gen_tables.py

# Fail if the generated tables have drifted from tools/defs/*.tsv.
tables-check:
    python3 tools/gen_tables.py --check

# Verify the built object: exported factory symbol, no unresolved symbols.
check: build tables-check
    @nm -D --defined-only {{shared_lib}} | grep -q fcitx_addon_factory_instance_vnkey \
        && echo "factory symbol: ok"
    @if ldd -r {{shared_lib}} 2>&1 | grep -q "undefined symbol"; then \
        echo "undefined symbols: FOUND"; exit 1; \
    else echo "undefined symbols: none"; fi

clean:
    cargo clean
    rm -rf {{out_dir}} build
