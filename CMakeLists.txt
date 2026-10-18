cmake_minimum_required(VERSION 3.16)

project(vnkey VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(Fcitx5Core REQUIRED)
find_package(Fcitx5Config REQUIRED)
find_package(Fcitx5Utils REQUIRED)

# ---------------------------------------------------------------------------
# Rust static library: engine + addon logic + cxx bridge + façade objects.
# `cargo build --features ffi` compiles cpp/facade.cpp as well, as long as the
# fcitx5 headers are present; crates/vnkey-addon/build.rs reports when they
# are not (and then only the Rust bridge is built).
# ---------------------------------------------------------------------------
set(CARGO_TARGET_DIR "${CMAKE_CURRENT_BINARY_DIR}/cargo")
set(RUST_STATIC_LIB "${CARGO_TARGET_DIR}/release/libvnkey_addon.a")

add_custom_command(
    OUTPUT "${RUST_STATIC_LIB}"
    COMMAND ${CMAKE_COMMAND} -E env
            "CARGO_TARGET_DIR=${CARGO_TARGET_DIR}"
            cargo build --release --features ffi
            --manifest-path "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    DEPENDS
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
        "${CMAKE_CURRENT_SOURCE_DIR}/crates/vnkey-core/src/lib.rs"
        "${CMAKE_CURRENT_SOURCE_DIR}/crates/vnkey-addon/src/lib.rs"
        "${CMAKE_CURRENT_SOURCE_DIR}/crates/vnkey-addon/src/bridge.rs"
        "${CMAKE_CURRENT_SOURCE_DIR}/crates/vnkey-addon/cpp/facade.cpp"
        "${CMAKE_CURRENT_SOURCE_DIR}/crates/vnkey-addon/cpp/facade.h"
    WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
    COMMENT "Building the Rust engine and façade"
    VERBATIM)

add_custom_target(vnkey_rust ALL DEPENDS "${RUST_STATIC_LIB}")

# The addon factory lives in the façade object inside the static archive, so the
# archive has to be linked whole or the symbol is dropped.
file(WRITE "${CMAKE_CURRENT_BINARY_DIR}/link-stub.cpp"
     "// Intentionally empty: the addon code comes from the Rust archive.\n")
add_library(vnkey SHARED "${CMAKE_CURRENT_BINARY_DIR}/link-stub.cpp")
add_dependencies(vnkey vnkey_rust)
# Order matters: Debian links with --as-needed, so the archive must come before
# the fcitx5 libraries or they get dropped as unused.
target_link_libraries(vnkey PRIVATE
    "-Wl,--whole-archive" "${RUST_STATIC_LIB}" "-Wl,--no-whole-archive"
    "-Wl,--no-as-needed"
    Fcitx5::Core
    Fcitx5::Config
    Fcitx5::Utils
    "-Wl,--no-undefined"
    "-Wl,-s"
)
set_target_properties(vnkey PROPERTIES
    OUTPUT_NAME vnkey
    INSTALL_RPATH "$ORIGIN")

configure_file("${CMAKE_CURRENT_SOURCE_DIR}/data/vnkey-addon.conf.in"
               vnkey-addon.conf @ONLY)
configure_file("${CMAKE_CURRENT_SOURCE_DIR}/data/vnkey.conf.in"
               vnkey.conf @ONLY)

include(GNUInstallDirs)
install(TARGETS vnkey
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}/fcitx5)
install(FILES "${CMAKE_CURRENT_BINARY_DIR}/vnkey-addon.conf"
    DESTINATION ${CMAKE_INSTALL_DATADIR}/fcitx5/addon
    RENAME vnkey.conf)
install(FILES "${CMAKE_CURRENT_BINARY_DIR}/vnkey.conf"
    DESTINATION ${CMAKE_INSTALL_DATADIR}/fcitx5/inputmethod)

# Input-method icon, referenced as `Icon=fcitx_vnkey` in vnkey.conf.
#
# Both formats are installed on purpose: fcitx5's own UI renders icons with
# libpng/libjpeg and has no SVG decoder, while Qt/GTK hosts look the name up in
# the theme and prefer the scalable file. fcitx5 picks the exact-size PNG when
# one exists, so a PNG-only setup would look blurry in the panel and an SVG-only
# setup shows nothing at all in fcitx5's status area.
install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/data/fcitx_vnkey.svg"
    DESTINATION ${CMAKE_INSTALL_DATADIR}/icons/hicolor/scalable/apps
    RENAME fcitx_vnkey.svg)

set(VNKEY_ICON_SIZES 16 22 24 32 48 64 128 256)
foreach(size ${VNKEY_ICON_SIZES})
    install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/data/icons/${size}/fcitx_vnkey.png"
        DESTINATION ${CMAKE_INSTALL_DATADIR}/icons/hicolor/${size}x${size}/apps
        RENAME fcitx_vnkey.png)
endforeach()
