/*
 * VnKey — fcitx5 facade implementation.
 *
 * SPDX-FileCopyrightText: 2022-2022 CSSlayer <wengxt@gmail.com>
 * SPDX-FileCopyrightText: 2026 Bamboo Rust port contributors
 * SPDX-License-Identifier: LGPL-2.1-or-later
 */
#include "facade.h"

#include <fcitx-utils/capabilityflags.h>
#include <fcitx-utils/keysym.h>
#include <fcitx-utils/log.h>
#include <fcitx-utils/textformatflags.h>
#include <fcitx-utils/utf8.h>
#include <fcitx/text.h>

#include <cstdlib>
#include <utility>

namespace fcitx {

namespace {

constexpr std::string_view MacroPrefix = "macro/";
const std::string MacroFile = "conf/vnkey-macro.conf";

FCITX_DEFINE_LOG_CATEGORY(vnkey, "vnkey");
#define FCITX_BAMBOORS_DEBUG() FCITX_LOGC(vnkey, Debug)

std::string toString(rust::String value) { return {value.c_str()}; }

rust::Str toStr(const std::string &value) {
    return rust::Str(value.data(), value.size());
}

} // namespace

// =========================================================================
// VnKeyState
// =========================================================================

VnKeyState::VnKeyState(VnKeyEngine *engine, InputContext *ic)
    : owner_(engine), ic_(ic) {
    setEngine();
}

void VnKeyState::setEngine() {
    engine_.reset();

    rust::Vec<vnkey::MacroPair> macros;
    for (const auto &entry : *owner_->macroTable().macros) {
        vnkey::MacroPair pair;
        pair.key = rust::String(entry.key->data());
        pair.value = rust::String(entry.value->data());
        macros.push_back(std::move(pair));
    }

    // An empty path means: use the dictionary embedded in the Rust library.
    engine_.emplace(vnkey::create(
        toStr(*owner_->config().inputMethod), std::move(macros), rust::Str()));
    if (!(*engine_)->valid()) {
        FCITX_ERROR() << "Bamboors: unknown input method "
                      << *owner_->config().inputMethod;
    }
    setOption();
}

void VnKeyState::setOption() {
    if (!engine_.has_value()) {
        return;
    }
    const auto &config = owner_->config();
    vnkey::EngineOptions options;
    options.auto_non_vn_restore = *config.autoNonVnRestore;
    options.macro_enabled = *config.macro;
    options.auto_capitalize_macro = *config.capitalizeMacro;
    options.spell_check_with_dicts = *config.spellCheck;
    options.modern_style = *config.modernStyle;
    options.free_marking = *config.freeMarking;
    options.output_charset = rust::String(config.outputCharset->data());
    (*engine_)->set_options(std::move(options));
    (*engine_)->set_vietnamese_mode(*config.vietnameseMode);
}

void VnKeyState::keyEvent(KeyEvent &keyEvent) {
    if (!ready()) {
        return;
    }
    // Ignore all key releases and bare Shift.
    if (keyEvent.isRelease()) {
        return;
    }
    if (keyEvent.rawKey().check(FcitxKey_Shift_L) ||
        keyEvent.rawKey().check(FcitxKey_Shift_R)) {
        return;
    }

    if (keyEvent.key().checkKeyList(*owner_->config().switchKey)) {
        owner_->toggleVietnameseMode(ic_);
        keyEvent.filterAndAccept();
        return;
    }

    if (keyEvent.key().checkKeyList(*owner_->config().restoreKeyStroke)) {
        (*engine_)->set_restore_key_stroke();
        keyEvent.filterAndAccept();
        return;
    }

    const uint32_t sym = keyEvent.rawKey().sym();
    const uint32_t states = keyEvent.rawKey().states();
    if ((*engine_)->process_key_event(sym, states)) {
        keyEvent.filterAndAccept();
    }

    const auto commit = (*engine_)->pull_commit();
    if (!commit.empty()) {
        ic_->commitString(toString(commit));
    }

    ic_->inputPanel().reset();
    const auto preedit = (*engine_)->pull_preedit();
    if (!preedit.empty()) {
        const std::string preeditUtf8 = toString(preedit);
        // Legacy charsets (TCVN3, VISCII, …) are not UTF-8; never hand them to
        // the input panel as text.
        if (utf8::validate(preeditUtf8)) {
            Text text;
            TextFormatFlags format;
            if (ic_->capabilityFlags().test(CapabilityFlag::Preedit) &&
                *owner_->config().displayUnderline) {
                format = TextFormatFlag::Underline;
            }
            text.append(preeditUtf8, format);
            text.setCursor(text.textLength());
            if (ic_->capabilityFlags().test(CapabilityFlag::Preedit)) {
                ic_->inputPanel().setClientPreedit(text);
            } else {
                ic_->inputPanel().setPreedit(text);
            }
        }
    }
    ic_->updatePreedit();
    ic_->updateUserInterface(UserInterfaceComponent::InputPanel);
}

void VnKeyState::reset() {
    ic_->inputPanel().reset();
    if (engine_.has_value()) {
        (*engine_)->reset();
    }
    ic_->updateUserInterface(UserInterfaceComponent::InputPanel);
    ic_->updatePreedit();
}

void VnKeyState::commitBuffer() {
    ic_->inputPanel().reset();
    if (engine_.has_value()) {
        // Commit explicitly: when a client loses focus the framework may not
        // commit the preedit for us.
        (*engine_)->commit_preedit();
        const auto commit = (*engine_)->pull_commit();
        if (!commit.empty()) {
            ic_->commitString(toString(commit));
        }
    }
    ic_->updateUserInterface(UserInterfaceComponent::InputPanel);
    ic_->updatePreedit();
}

// =========================================================================
// VnKeyEngine
// =========================================================================

VnKeyEngine::VnKeyEngine(Instance *instance)
    : instance_(instance), factory_([this](InputContext &ic) {
          return new VnKeyState(this, &ic);
      }) {
    for (const auto &name : vnkey::input_method_names()) {
        imNames_.push_back(toString(name));
    }
    if (imNames_.empty()) {
        throw std::runtime_error("Bamboors: no input methods available");
    }
    config_.inputMethod.annotation().setList(imNames_);
    FCITX_BAMBOORS_DEBUG() << "Input methods: " << imNames_;

    for (const auto &name : vnkey::charset_names()) {
        charsets_.push_back(toString(name));
    }
    config_.outputCharset.annotation().setList(charsets_);

    auto &uiManager = instance_->userInterfaceManager();

    inputMethodAction_ = std::make_unique<SimpleAction>();
    inputMethodAction_->setIcon("document-edit");
    inputMethodAction_->setShortText(_("Input Method"));
    uiManager.registerAction("vnkey-input-method", inputMethodAction_.get());
    inputMethodMenu_ = std::make_unique<Menu>();
    inputMethodAction_->setMenu(inputMethodMenu_.get());
    for (const auto &name : imNames_) {
        inputMethodSubAction_.emplace_back(std::make_unique<SimpleAction>());
        auto *action = inputMethodSubAction_.back().get();
        action->setShortText(name);
        action->setCheckable(true);
        uiManager.registerAction(stringutils::concat("vnkey-im-", name),
                                 action);
        connections_.emplace_back(action->connect<SimpleAction::Activated>(
            [this, name](InputContext *ic) {
                if (config_.inputMethod.value() == name) {
                    return;
                }
                config_.inputMethod.setValue(name);
                saveConfig();
                refreshEngine();
                updateInputMethodAction(ic);
            }));
        inputMethodMenu_->addAction(action);
    }

    charsetAction_ = std::make_unique<SimpleAction>();
    charsetAction_->setIcon("character-set");
    charsetAction_->setShortText(_("Output Charset"));
    uiManager.registerAction("vnkey-charset", charsetAction_.get());
    charsetMenu_ = std::make_unique<Menu>();
    charsetAction_->setMenu(charsetMenu_.get());
    for (const auto &name : charsets_) {
        charsetSubAction_.emplace_back(std::make_unique<SimpleAction>());
        auto *action = charsetSubAction_.back().get();
        action->setShortText(name);
        action->setCheckable(true);
        uiManager.registerAction(stringutils::concat("vnkey-cs-", name),
                                 action);
        connections_.emplace_back(action->connect<SimpleAction::Activated>(
            [this, name](InputContext *ic) {
                if (config_.outputCharset.value() == name) {
                    return;
                }
                config_.outputCharset.setValue(name);
                saveConfig();
                refreshOption();
                updateCharsetAction(ic);
            }));
        charsetMenu_->addAction(action);
    }

    modeAction_ = std::make_unique<SimpleAction>();
    modeAction_->setCheckable(true);
    connections_.emplace_back(modeAction_->connect<SimpleAction::Activated>(
        [this](InputContext *ic) { toggleVietnameseMode(ic); }));
    uiManager.registerAction("vnkey-mode", modeAction_.get());

    spellCheckAction_ = std::make_unique<SimpleAction>();
    spellCheckAction_->setIcon("tools-check-spelling");
    connections_.emplace_back(
        spellCheckAction_->connect<SimpleAction::Activated>(
            [this](InputContext *ic) {
                config_.spellCheck.setValue(!*config_.spellCheck);
                saveConfig();
                refreshOption();
                updateSpellAction(ic);
            }));
    uiManager.registerAction("vnkey-spell-check", spellCheckAction_.get());

    macroAction_ = std::make_unique<SimpleAction>();
    macroAction_->setIcon("edit-find");
    connections_.emplace_back(macroAction_->connect<SimpleAction::Activated>(
        [this](InputContext *ic) {
            config_.macro.setValue(!*config_.macro);
            saveConfig();
            refreshOption();
            updateMacroAction(ic);
        }));
    uiManager.registerAction("vnkey-macro", macroAction_.get());

    reloadConfig();
    instance_->inputContextManager().registerProperty("vnkeyState",
                                                      &factory_);
}

void VnKeyEngine::reloadConfig() {
    readAsIni(config_, "conf/vnkey.conf");
    readAsIni(macroTable_, MacroFile);
    populateConfig();
}

void VnKeyEngine::setConfig(const RawConfig &config) {
    config_.load(config, true);
    saveConfig();
    populateConfig();
}

const Configuration *VnKeyEngine::getSubConfig(const std::string &path) const {
    if (path == "macro") {
        return &macroTable_;
    }
    return nullptr;
}

void VnKeyEngine::setSubConfig(const std::string &path,
                                const RawConfig &config) {
    if (path == "macro") {
        macroTable_.load(config, true);
        safeSaveAsIni(macroTable_, MacroFile);
        refreshEngine();
    }
}

void VnKeyEngine::populateConfig() {
    refreshEngine();
    refreshOption();
    updateInputMethodAction(nullptr);
    updateCharsetAction(nullptr);
    updateSpellAction(nullptr);
    updateMacroAction(nullptr);
    updateModeAction(nullptr);
}

std::string VnKeyEngine::subMode(const InputMethodEntry & /*entry*/,
                                  InputContext & /*inputContext*/) {
    return *config_.inputMethod;
}

std::string VnKeyEngine::overrideIcon(const InputMethodEntry & /*entry*/) {
    return *config_.vietnameseMode ? "fcitx_vnkey" : "fcitx_vnkey_en";
}

void VnKeyEngine::activate(const InputMethodEntry & /*entry*/,
                            InputContextEvent &event) {
    auto &statusArea = event.inputContext()->statusArea();
    updateInputMethodAction(event.inputContext());
    updateCharsetAction(event.inputContext());
    updateSpellAction(event.inputContext());
    updateMacroAction(event.inputContext());
    updateModeAction(event.inputContext());

    statusArea.addAction(StatusGroup::InputMethod, modeAction_.get());
    statusArea.addAction(StatusGroup::InputMethod, inputMethodAction_.get());
    statusArea.addAction(StatusGroup::InputMethod, charsetAction_.get());
    statusArea.addAction(StatusGroup::InputMethod, spellCheckAction_.get());
    statusArea.addAction(StatusGroup::InputMethod, macroAction_.get());
}

void VnKeyEngine::deactivate(const InputMethodEntry & /*entry*/,
                              InputContextEvent &event) {
    auto *state = event.inputContext()->propertyFor(&factory_);
    if (event.type() != EventType::InputContextFocusOut) {
        state->commitBuffer();
    } else {
        state->reset();
    }
}

void VnKeyEngine::keyEvent(const InputMethodEntry & /*entry*/,
                            KeyEvent &keyEvent) {
    auto *state = keyEvent.inputContext()->propertyFor(&factory_);
    state->keyEvent(keyEvent);
}

void VnKeyEngine::reset(const InputMethodEntry & /*entry*/,
                         InputContextEvent &event) {
    auto *state = event.inputContext()->propertyFor(&factory_);
    state->reset();
}

void VnKeyEngine::refreshEngine() {
    if (!factory_.registered()) {
        return;
    }
    instance_->inputContextManager().foreach([this](InputContext *ic) {
        auto *state = ic->propertyFor(&factory_);
        state->setEngine();
        if (ic->hasFocus()) {
            state->reset();
        }
        return true;
    });
}

void VnKeyEngine::refreshOption() {
    if (!factory_.registered()) {
        return;
    }
    instance_->inputContextManager().foreach([this](InputContext *ic) {
        auto *state = ic->propertyFor(&factory_);
        state->setOption();
        if (ic->hasFocus()) {
            state->reset();
        }
        return true;
    });
}

void VnKeyEngine::updateInputMethodAction(InputContext *ic) {
    const auto expected = stringutils::concat("vnkey-im-", *config_.inputMethod);
    for (const auto &action : inputMethodSubAction_) {
        action->setChecked(action->name() == expected);
        if (ic) {
            action->update(ic);
        }
    }
}

void VnKeyEngine::updateCharsetAction(InputContext *ic) {
    const auto expected = stringutils::concat("vnkey-cs-", *config_.outputCharset);
    for (const auto &action : charsetSubAction_) {
        action->setChecked(action->name() == expected);
        if (ic) {
            action->update(ic);
        }
    }
}

void VnKeyEngine::updateSpellAction(InputContext *ic) {
    spellCheckAction_->setChecked(*config_.spellCheck);
    spellCheckAction_->setShortText(*config_.spellCheck
                                        ? _("Spell Check Enabled")
                                        : _("Spell Check Disabled"));
    if (ic) {
        spellCheckAction_->update(ic);
    }
}

void VnKeyEngine::toggleVietnameseMode(InputContext *ic) {
    const bool next = !*config_.vietnameseMode;
    config_.vietnameseMode.setValue(next);
    saveConfig();

    // Global by construction: config_ belongs to the addon, not to an input
    // context, so every engine sees the same value. Flush each pending word
    // before flipping, otherwise a half-typed syllable disappears.
    if (factory_.registered()) {
        instance_->inputContextManager().foreach([this](InputContext *context) {
            auto *state = context->propertyFor(&factory_);
            state->commitBuffer();
            state->setOption();
            state->reset();
            return true;
        });
    }

    updateModeAction(ic);
    if (ic) {
        ic->updateUserInterface(UserInterfaceComponent::StatusArea);
    }
}

void VnKeyEngine::updateModeAction(InputContext *ic) {
    const bool vietnamese = *config_.vietnameseMode;
    modeAction_->setChecked(vietnamese);
    modeAction_->setIcon(vietnamese ? "fcitx_vnkey" : "fcitx_vnkey_en");
    modeAction_->setShortText(vietnamese ? _("Vietnamese") : _("English"));
    if (ic) {
        modeAction_->update(ic);
    }
}

void VnKeyEngine::updateMacroAction(InputContext *ic) {
    macroAction_->setChecked(*config_.macro);
    macroAction_->setShortText(*config_.macro ? _("Macro Enabled")
                                              : _("Macro Disabled"));
    if (ic) {
        macroAction_->update(ic);
    }
}

} // namespace fcitx

FCITX_ADDON_FACTORY_V2(vnkey, fcitx::VnKeyFactory)
