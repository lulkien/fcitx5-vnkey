//! Behaviour tests: every expectation is pinned key by key.

use vnkey_core::{parse_input_method, Engine, Flags, Mode};

fn std_engine() -> Engine {
    Engine::new(parse_input_method("Telex 2").expect("Telex 2"), Flags::STD)
}

fn engine_for(name: &str) -> Engine {
    Engine::new(parse_input_method(name).expect(name), Flags::STD)
}

fn vie(engine: &Engine) -> String {
    engine.get_processed_string(Mode::VIETNAMESE)
}

fn eng(engine: &Engine) -> String {
    engine.get_processed_string(Mode::ENGLISH)
}

#[test]
fn process_string() {
    let mut ng = std_engine();
    ng.process_string("aw", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "ă");

    ng.reset();
    ng.process_string("uw", Mode::VIETNAMESE);
    ng.process_string("o", Mode::VIETNAMESE);
    ng.process_string("w", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "ươ");

    ng.reset();
    ng.process_string("chuaarn", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "chuẩn");

    ng.reset();
    ng.process_string("giamaf", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "giầm");
}

#[test]
fn process_dd_string() {
    let mut ng = std_engine();
    ng.process_string("dd", Mode::VIETNAMESE);
    assert!(ng.is_valid(false));

    ng.reset();
    ng.process_string("ddafi", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "đài");
}

#[test]
fn process_muoiwq_string() {
    let mut ng = std_engine();
    ng.process_string("Muoiwq", Mode::VIETNAMESE);
    assert_eq!(eng(&ng), "Muoiwq");

    ng.reset();
    ng.process_string("mootj", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "một");
}

#[test]
fn process_thuow_string() {
    let mut ng = std_engine();
    ng.process_string("Thuow", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "Thuơ");

    ng.remove_last_char(true);
    assert_eq!(vie(&ng), "Thu");
}

#[test]
fn remove_last_char_before_word() {
    let mut ng = std_engine();
    ng.remove_last_char(true);
    ng.process_string(" ", Mode::ENGLISH);
    ng.remove_last_char(true);
    ng.process_string("loanj", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "loạn");

    ng.remove_last_char(true);
    assert_eq!(vie(&ng), "lọa");

    ng.process_string(":", Mode::ENGLISH);
    ng.remove_last_char(true);
    assert_eq!(vie(&ng), "lọa");
}

#[test]
fn process_upper_string() {
    let mut ng = std_engine();
    ng.process_string("VIEETJ", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "VIỆT");

    ng.remove_last_char(false);
    assert_eq!(vie(&ng), "VIỆ");

    ng.process_key('Q', Mode::VIETNAMESE);
    assert_eq!(eng(&ng), "VIEEJQ");

    ng.reset();
    ng.process_string("IB", Mode::ENGLISH);
    assert_eq!(eng(&ng), "IB");
}

#[test]
fn spelling_check() {
    let mut ng = std_engine();
    ng.process_string("noww", Mode::VIETNAMESE);
    assert_eq!(eng(&ng), "noww");
    assert_eq!(vie(&ng), "now");

    ng.reset();
    ng.process_string("sawss", Mode::VIETNAMESE);
    assert_eq!(eng(&ng), "sawss");

    ng.reset();
    ng.process_string("sawss", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "săs");
}

#[test]
fn process_dd() {
    let mut ng = std_engine();
    ng.process_string("dd", Mode::VIETNAMESE);
    assert!(ng.is_valid(false));
    assert_eq!(vie(&ng), "đ");

    ng.reset();
    ng.process_string("SD", Mode::VIETNAMESE);
    ng.process_string("D", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "SĐ");
}

#[test]
fn telex_23() {
    let mut ng = std_engine();
    ng.process_string("t ]", Mode::ENGLISH);
    ng.process_string("a", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "]a");

    ng.reset();
    ng.process_string("]]a", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "]a");

    let mut ng = engine_for("Telex 2");
    ng.process_string("[", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "ơ");

    ng.reset();
    ng.process_string("{", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "Ơ");
}

#[test]
fn process_nguwowfi_string() {
    let mut ng = std_engine();
    ng.process_string("wowfi", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "ười");
}

#[test]
fn remove_last_char() {
    let mut ng = std_engine();
    ng.process_string("hanhj", Mode::VIETNAMESE);
    ng.remove_last_char(true);
    assert_eq!(vie(&ng), "hạn");
}

#[test]
fn process_catr_string() {
    let mut ng = std_engine();
    ng.process_string("catr", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "catr");
}

#[test]
fn process_toowi_string() {
    let mut ng = std_engine();
    ng.process_string("toowi", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "tơi");
}

#[test]
fn process_aloo_string() {
    let mut ng = std_engine();
    ng.process_string("aloo", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "alô");
}

#[test]
fn spelling_check_for_giw() {
    let mut ng = std_engine();
    ng.process_string("giw", Mode::VIETNAMESE);
    assert!(ng.is_valid(false));
}

#[test]
fn double_brackets() {
    let mut ng = std_engine();
    ng.process_string("[[", Mode::VIETNAMESE);
    assert_eq!(eng(&ng), "[");
}

#[test]
fn double_brackets_o() {
    let mut ng = std_engine();
    ng.process_string("tooss", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "tôs");

    ng.reset();
    ng.process_string("tosos", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "tôs");
}

#[test]
fn double_w() {
    let mut ng = std_engine();
    ng.process_string("ww", Mode::VIETNAMESE);
    assert_eq!(eng(&ng), "w");
    assert_eq!(vie(&ng), "w");
}

#[test]
fn double_w2() {
    let mut ng = std_engine();
    ng.process_string("wiw", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "uiw");
    assert_eq!(eng(&ng), "wiw");
}

#[test]
fn process_duwoi() {
    let mut ng = std_engine();
    ng.process_string("duwoi", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "dươi");
}

#[test]
fn process_refresh() {
    let mut ng = std_engine();
    ng.process_string("reff", Mode::VIETNAMESE);
    ng.process_string("resh", Mode::ENGLISH);
    assert_eq!(eng(&ng), "reffresh");
    assert_eq!(vie(&ng), "refresh");
}

#[test]
fn process_refresh2() {
    let mut ng = std_engine();
    ng.process_string("reff", Mode::VIETNAMESE);
    ng.remove_last_char(true);
    ng.process_key('f', Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "rè");
}

#[test]
fn process_dd_seq() {
    let mut ng = std_engine();
    ng.process_string("oddp", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "ođp");
}

#[test]
fn process_gisa() {
    let mut ng = std_engine();
    ng.process_string("gis", Mode::VIETNAMESE);
    ng.process_string("a", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "giá");
}

#[test]
fn process_kimso() {
    let mut ng = std_engine();
    ng.process_string("kimso", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "kímo");
}

#[test]
fn process_to() {
    let mut ng = std_engine();
    ng.process_string("to", Mode::VIETNAMESE);
    assert!(ng.is_valid(true));
}

#[test]
fn process_toorr() {
    let mut ng = std_engine();
    ng.process_string("toorr", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "tôr");
}

#[test]
fn process_tnoss() {
    let mut ng = std_engine();
    ng.process_string("tnoss", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "tnos");
}

#[test]
fn process_eenghf() {
    let mut ng = engine_for("Telex 2");
    ng.process_string("ddawks", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "đắk");
}

#[test]
fn process_hieeur() {
    let mut ng = std_engine();
    ng.process_string("tooi oo HIEEUR", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "HIỂU");
}

#[test]
fn process_nguoiw() {
    let mut ng = std_engine();
    ng.process_string("NGUOIW", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "NGƯƠI");
}

#[test]
fn process_tos() {
    let mut ng = std_engine();
    ng.process_string("{s", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "Ớ");
}

#[test]
fn process_to5() {
    let mut ng = engine_for("VNI");
    ng.process_string("o55", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "o5");
}

#[test]
fn process_huoswc() {
    let mut ng = std_engine();
    ng.process_string("duwongwj", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "duongwj");
}

#[test]
fn process_choas() {
    let mut flags = Flags::STD;
    flags.set(Flags::STD_TONE_STYLE, false);
    let mut ng = Engine::new(parse_input_method("Telex 2").expect("Telex 2"), flags);

    ng.process_string("choas", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "choá");

    ng.reset();
    ng.process_string("bieecs", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "biếc");

    ng.reset();
    ng.process_string("uese", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "uế");
}

#[test]
fn restore_last_word() {
    let mut ng = std_engine();
    ng.process_string("duwongj tooi", Mode::VIETNAMESE);
    ng.restore_last_word(false);
    assert_eq!(vie(&ng), "tooi");
}

#[test]
fn restore_last_word_tcvn() {
    let mut ng = engine_for("Microsoft layout");
    ng.process_string("112", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "1â");

    ng.restore_last_word(false);
    assert_eq!(eng(&ng), "12");

    ng.reset();
    ng.process_string("d[]ng9 t4i", Mode::VIETNAMESE);
    ng.restore_last_word(false);
    assert_eq!(vie(&ng), "t4i");
}

#[test]
fn z_processing() {
    let mut ng = std_engine();
    ng.process_string("loz", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "loz");

    ng.reset();
    ng.process_string("losz", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "lo");
    assert_eq!(eng(&ng), "losz");
}

#[test]
fn restore_last_word_smoke() {
    let mut ng = std_engine();
    ng.process_string("afq", Mode::VIETNAMESE);
    ng.restore_last_word(false);
    ng.remove_last_char(true);
    ng.process_key('f', Mode::VIETNAMESE);
    // Upstream only logs the result here.
    let _ = (vie(&ng), eng(&ng));
}

#[test]
fn process_vn_word() {
    let mut ng = std_engine();
    ng.process_string("tôifs", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "tối");
    assert_eq!(eng(&ng), "tôifs");

    ng.reset();
    ng.process_string("tốif", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "tồi");
    assert_eq!(eng(&ng), "tốif");

    ng.reset();
    ng.process_string("tốiz", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "tôi");
}

#[test]
fn double_typing() {
    let mut ng = std_engine();

    ng.process_string("linux", Mode::VIETNAMESE);
    ng.process_string("x", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "linux");

    ng.reset();
    ng.process_string("buwo", Mode::VIETNAMESE);
    ng.process_string("o", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "buô");

    ng.reset();
    ng.process_string("buowc", Mode::VIETNAMESE);
    ng.process_string("o", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "buôc");

    ng.reset();
    ng.process_string("cuoiw", Mode::VIETNAMESE);
    ng.process_string("o", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "cuôi");

    ng.reset();
    ng.process_string("ach", Mode::VIETNAMESE);
    ng.process_string("a", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "acha");

    ng.reset();
    ng.process_string("nhuw", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "như");
    assert!(ng.is_valid(true));

    ng.reset();
    ng.process_string("thuw", Mode::VIETNAMESE);
    assert!(ng.is_valid(true));

    ng.reset();
    ng.process_string("thow", Mode::VIETNAMESE);
    assert!(ng.is_valid(true));

    ng.reset();
    ng.process_string("tooi", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "tôi");
    assert!(ng.is_valid(true));

    ng.reset();
    ng.process_string("arch", Mode::VIETNAMESE);
    assert!(!ng.is_valid(false));

    ng.reset();
    ng.process_string("[[", Mode::VIETNAMESE);
    ng.process_string("oo", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "[ô");

    ng.reset();
    ng.process_string("oo]", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "ôư");

    ng.reset();
    ng.process_string("chury", Mode::VIETNAMESE);
    assert!(ng.is_valid(true));

    ng.reset();
    ng.process_string("turyn", Mode::VIETNAMESE);
    ng.remove_last_char(true);
    ng.remove_last_char(true);
    assert_eq!(vie(&ng), "tủ");

    ng.reset();
    ng.process_string("chuyển", Mode::VIETNAMESE);
    ng.process_string("z", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "chuyên");

    ng.reset();
    ng.process_string("nhueej", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "nhuệ");

    ng.reset();
    ng.process_string("cuongw", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "cương");

    ng.reset();
    ng.process_string("quawcj", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "quặc");

    ng.reset();
    ng.process_string("quawcj", Mode::VIETNAMESE);
    assert_eq!(vie(&ng), "quặc");

    ng.reset();
    ng.process_string("tôi）t", Mode::ENGLISH);
    assert_eq!(vie(&ng), "t");
}
