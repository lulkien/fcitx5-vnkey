/*
 * VnKey — fcitx5 facade.
 *
 * SPDX-FileCopyrightText: 2022-2022 CSSlayer <wengxt@gmail.com>
 * SPDX-FileCopyrightText: 2026 Bamboo Rust port contributors
 * SPDX-License-Identifier: LGPL-2.1-or-later
 *
 * This file is the only C++ in the port. It owns nothing but fcitx5 types:
 * configuration, status-area actions, input-context properties, and the
 * preedit/commit rendering. Every key-handling decision lives in Rust behind
 * the cxx bridge declared in `src/bridge.rs`.
 */
#pragma once

#include <fcitx-config/configuration.h>
#include <fcitx-config/iniparser.h>
#include <fcitx-config/rawconfig.h>
#include <fcitx-utils/i18n.h>
#include <fcitx-utils/stringutils.h>
#include <fcitx/action.h>
#include <fcitx/addonfactory.h>
#include <fcitx/addoninstance.h>
#include <fcitx/addonmanager.h>
#include <fcitx/inputcontextproperty.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/instance.h>
#include <fcitx/menu.h>
#include <fcitx/statusarea.h>
#include <fcitx/userinterfacemanager.h>

#include <memory>
#include <optional>
#include <string>
#include <unordered_map>
#include <utility>
#include <vector>

#include "vnkey-addon/src/bridge.rs.h"

namespace fcitx {

/// List annotation that also publishes the list into the config description,
/// so KDE's fcitx5 KCM renders both options as combo boxes.
struct StringListAnnotation : public EnumAnnotation {
    void setList(std::vector<std::string> list) { list_ = std::move(list); }
    const auto &list() { return list_; }
    void dumpDescription(RawConfig &config) const {
        EnumAnnotation::dumpDescription(config);
        for (size_t i = 0; i < list_.size(); i++) {
            config.setValueByPath("Enum/" + std::to_string(i), list_[i]);
        }
    }

protected:
    std::vector<std::string> list_;
};

FCITX_CONFIGURATION(VnKeyKeymap,
                    Option<std::string> key{this, "Key", _("Key"), ""};
                    Option<std::string> value{this, "Value", _("Value"), ""};);

FCITX_CONFIGURATION(
    VnKeyMacroTable,
    OptionWithAnnotation<std::vector<VnKeyKeymap>, ListDisplayOptionAnnotation>
        macros{this,
               "Macro",
               _("Macro"),
               {},
               {},
               {},
               ListDisplayOptionAnnotation("Key")};);

FCITX_CONFIGURATION(
    VnKeyConfig, KeyListOption restoreKeyStroke{
                      this,
                      "RestoreKeyStroke",
                      _("Restore Key Stroke"),
                      {Key("Shift+space")},
                      KeyListConstrain()};
    OptionWithAnnotation<std::string, StringListAnnotation> inputMethod{
        this, "InputMethod", _("Input Method"), "Telex 2"};
    OptionWithAnnotation<std::string, StringListAnnotation> outputCharset{
        this, "OutputCharset", _("Output Charset"), "Unicode"};
    Option<bool> spellCheck{this, "SpellCheck", _("Enable spell check"), true};
    Option<bool> macro{this, "Macro", _("Enable Macro"), true};
    Option<bool> capitalizeMacro{this, "CapitalizeMacro", _("Capitalize Macro"),
                                 true};
    Option<bool> autoNonVnRestore{this, "AutoNonVnRestore",
                                  _("Auto restore keys with invalid words"),
                                  true};
    Option<bool> modernStyle{this, "ModernStyle",
                             _("Use oà, _uý (instead of òa, úy)"), false};
    Option<bool> freeMarking{this, "FreeMarking",
                             _("Allow type with more freedom"), true};
    Option<bool> displayUnderline{this, "DisplayUnderline",
                                  _("Underline the preedit text"), true};
    Option<bool> vietnameseMode{this, "VietnameseMode",
                                _("Vietnamese mode (off = English)"), true};
    KeyListOption switchKey{this,
                            "SwitchKey",
                            _("Switch Vietnamese/English mode"),
                            {Key("Control+Shift+space")},
                            KeyListConstrain()};
    SubConfigOption macroTable{this, "MacroTable", _("Macro"),
                               "fcitx://config/addon/vnkey/macro"};);

class VnKeyEngine;

/// Per-input-context state: one Rust engine each.
class VnKeyState final : public InputContextProperty {
public:
    VnKeyState(VnKeyEngine *engine, InputContext *ic);
    ~VnKeyState() override = default;

    void setEngine();
    void setOption();
    void keyEvent(KeyEvent &keyEvent);
    void reset();
    void commitBuffer();
    /// Commit (focused) or drop (unfocused) the pending word, then reset.
    void flush();
    /// Redraw the panel from the engine's preedit, if there is one.
    void repaint();

    bool ready() const { return engine_.has_value() && (*engine_)->valid(); }

private:
    void render(const rust::String &preedit);

    VnKeyEngine *owner_;
    InputContext *ic_;
    std::optional<rust::Box<vnkey::EngineHandle>> engine_;
};

class VnKeyEngine final : public InputMethodEngine {
public:
    explicit VnKeyEngine(Instance *instance);

    void activate(const InputMethodEntry &entry,
                  InputContextEvent &event) override;
    void deactivate(const fcitx::InputMethodEntry &entry,
                    fcitx::InputContextEvent &event) override;
    void keyEvent(const InputMethodEntry &entry, KeyEvent &keyEvent) override;
    void reset(const InputMethodEntry &entry,
               InputContextEvent &event) override;
    std::string subMode(const InputMethodEntry &entry,
                        InputContext &inputContext) override;
    std::string overrideIcon(const InputMethodEntry &entry) override;

    const Configuration *getConfig() const override { return &config_; }
    void setConfig(const RawConfig &config) override;
    const Configuration *getSubConfig(const std::string &path) const override;
    void setSubConfig(const std::string &path,
                      const RawConfig &config) override;
    void reloadConfig() override;

    void refreshEngine();
    void refreshOption();
    void saveConfig() { safeSaveAsIni(config_, "conf/vnkey.conf"); }

    const VnKeyConfig &config() const { return config_; }
    const VnKeyMacroTable &macroTable() const { return macroTable_; }

    void updateInputMethodAction(InputContext *ic);
    void updateCharsetAction(InputContext *ic);
    void updateSpellAction(InputContext *ic);
    void updateMacroAction(InputContext *ic);
    void updateModeAction(InputContext *ic);

    /// Flip the daemon-wide Vietnamese/English flag, flush every pending
    /// word and push the new state into all live engines.
    void toggleVietnameseMode(InputContext *ic);

private:
    /// Replace an unoffered output charset with Unicode; true if it changed.
    bool normalizeCharset();
    void populateConfig();

    Instance *instance_;
    VnKeyConfig config_;
    // What the live engines were last built or flushed for; see populateConfig.
    std::string appliedInputMethod_;
    std::string appliedCharset_;
    std::string warnedCharset_;
    bool appliedVietnameseMode_ = true;
    std::vector<std::pair<std::string, std::string>> appliedMacros_;
    VnKeyMacroTable macroTable_;
    FactoryFor<VnKeyState> factory_;
    std::vector<std::string> imNames_;
    std::vector<std::string> charsets_;

    std::unique_ptr<SimpleAction> inputMethodAction_;
    std::vector<std::unique_ptr<SimpleAction>> inputMethodSubAction_;
    std::unique_ptr<Menu> inputMethodMenu_;
    std::unique_ptr<SimpleAction> charsetAction_;
    std::vector<std::unique_ptr<SimpleAction>> charsetSubAction_;
    std::unique_ptr<Menu> charsetMenu_;
    std::unique_ptr<SimpleAction> spellCheckAction_;
    std::unique_ptr<SimpleAction> modeAction_;
    std::unique_ptr<SimpleAction> macroAction_;
    std::vector<ScopedConnection> connections_;
};

class VnKeyFactory : public AddonFactory {
public:
    AddonInstance *create(AddonManager *manager) override {
#ifdef FCITX_INSTALL_LOCALEDIR
        registerDomain("vnkey", FCITX_INSTALL_LOCALEDIR);
#endif
        return new VnKeyEngine(manager->instance());
    }
};

} // namespace fcitx
