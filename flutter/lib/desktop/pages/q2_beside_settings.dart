import 'package:bot_toast/bot_toast.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/desktop/pages/q2_canvas_editor.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:get/get.dart';

/// Settings tab for q2 Beside placement (R-INPUT-4, R-INPUT-5, R-INPUT-6).
///
/// Every option maps directly to a `Config::get_option` / `set_option` key on
/// the Rust side.  The control thread in `q2_beside.rs` re-reads them every
/// second, so changes take effect without a restart.

const double _kCardFixedWidth = 540;
const double _kCardLeftMargin = 15;
const double _kContentHMargin = 15;
const double _kCheckBoxLeftMargin = 10;
const double _kListViewBottomMargin = 15;
const double _kTitleFontSize = 20;

class Q2BesideSettings extends StatefulWidget {
  const Q2BesideSettings({Key? key}) : super(key: key);

  @override
  State<Q2BesideSettings> createState() => _Q2BesideSettingsState();
}

class _Q2BesideSettingsState extends State<Q2BesideSettings> {
  @override
  Widget build(BuildContext context) {
    final scrollController = ScrollController();
    return ListView(
      controller: scrollController,
      children: [
        _canvasEditorCard(context),
        _placements(context),
        _guards(context),
        _hotkeys(context),
        _cursor(context),
        _wakeOnLan(context),
      ],
    ).marginOnly(bottom: _kListViewBottomMargin);
  }

  // ---------------------------------------------------------------------------
  // Canvas Editor (R-UI-2, R-LAYOUT-1)
  // ---------------------------------------------------------------------------

  Widget _canvasEditorCard(BuildContext context) {
    return _card(title: 'Physical Canvas Editor', children: [
      Container(
        height: 360,
        margin: const EdgeInsets.symmetric(
            horizontal: _kContentHMargin, vertical: 8),
        decoration: BoxDecoration(
          border: Border.all(color: Theme.of(context).dividerColor),
          borderRadius: BorderRadius.circular(8),
        ),
        clipBehavior: Clip.antiAlias,
        child: const Q2CanvasEditor(),
      ),
    ]);
  }

  // ---------------------------------------------------------------------------
  // Placements & Video (R-PLACE-1, R-PLACE-2, R-VIDEO-1, R-VIDEO-2, R-VIDEO-3)
  // ---------------------------------------------------------------------------

  Widget _placements(BuildContext context) {
    return _card(title: 'Placement & Video (Inside / Beside)', children: [
      _textOption(
        context,
        label: 'Placement mode',
        key: 'q2-placement-default',
        hint: 'beside, inside-windowed, inside-fullscreen',
        helperText:
            'Default placement for paired peers (beside, inside-windowed, inside-fullscreen)',
      ),
      _boolOption(
        context,
        'Auto-connect to discovered peers on startup',
        'q2-auto-connect',
        defaultValue: true,
      ),
      _boolOption(
        context,
        '4:4:4 true color mode for sharp text (VP9/AV1)',
        'q2-inside-444',
        defaultValue: true,
      ),
      _boolOption(
        context,
        'Display each remote monitor in its own window',
        'displays-as-individual-windows',
      ),
      _boolOption(
        context,
        'Use all local displays for fullscreen remote session',
        'use-all-my-displays-for-the-remote-session',
      ),
      _boolOption(
        context,
        'Forward peer audio to this computer (default for Inside)',
        'q2-forward-audio',
      ),
    ]);
  }

  // ---------------------------------------------------------------------------
  // Guards (R-INPUT-4)
  // ---------------------------------------------------------------------------

  Widget _guards(BuildContext context) {
    return _card(title: 'Switch guards', children: [
      _deadZone(context),
      _boolOption(context, 'Require modifier to cross', 'q2-require-modifier'),
      _boolOption(context, 'Block while fullscreen', 'q2-block-fullscreen'),
      _textOption(
        context,
        label: 'Fullscreen ignore list',
        key: 'q2-fullscreen-ignore',
        hint: 'app1.exe, app2.exe',
        helperText: 'Comma-separated exe names (e.g. vlc.exe)',
      ),
    ]);
  }

  Widget _deadZone(BuildContext context) {
    final stored = bind.mainGetOptionSync(key: 'q2-corner-dead-zone-mm');
    final controller = TextEditingController(text: stored);
    final typed = RxString(stored);
    final saved = RxString(stored);

    return Row(
      children: [
        Text(
          '${translate("Corner dead zone (mm)")}: ',
          style: const TextStyle(fontSize: 14),
        ),
        SizedBox(
          width: 80,
          child: TextField(
            controller: controller,
            onChanged: (v) => typed.value = v,
            inputFormatters: [
              FilteringTextInputFormatter.allow(RegExp(r'^[0-9]{0,3}$')),
            ],
            decoration: const InputDecoration(
              hintText: '0',
              contentPadding:
                  EdgeInsets.symmetric(vertical: 12, horizontal: 12),
            ),
          ).workaroundFreezeLinuxMint().marginOnly(right: 15),
        ),
        Obx(() => Offstage(
              offstage: typed.value.trim() == saved.value.trim(),
              child: ElevatedButton(
                onPressed: () async {
                  final v = controller.text.trim();
                  await bind.mainSetOption(
                      key: 'q2-corner-dead-zone-mm', value: v);
                  typed.value = v;
                  saved.value = v;
                },
                child: Text(translate('Apply')),
              ),
            )),
      ],
    ).marginOnly(left: _kContentHMargin);
  }

  // ---------------------------------------------------------------------------
  // Hotkeys (R-INPUT-5)
  // ---------------------------------------------------------------------------

  Widget _hotkeys(BuildContext context) {
    return _card(title: 'Hotkeys', children: [
      _textOption(
        context,
        label: 'Return home',
        key: 'q2-hotkey-home',
        hint: 'ctrl+alt+shift+escape',
        helperText: 'Shortcut to release cursor to controller (default: ctrl+alt+shift+escape)',
      ),
      _textOption(
        context,
        label: 'Jump to peer',
        key: 'q2-hotkey-peer',
        hint: 'ctrl+alt+f1',
        helperText: 'Shortcut to move cursor to peer (e.g. ctrl+alt+f1)',
      ),
    ]);
  }

  // ---------------------------------------------------------------------------
  // Cursor overlay (R-INPUT-6)
  // ---------------------------------------------------------------------------

  Widget _cursor(BuildContext context) {
    return _card(title: 'Cursor', children: [
      _boolOption(
          context, 'Draw cursor on mouseless target', 'q2-draw-cursor'),
    ]);
  }

  // ---------------------------------------------------------------------------
  // Wake-on-LAN (R-WAKE-1, R-WAKE-2)
  // ---------------------------------------------------------------------------

  Widget _wakeOnLan(BuildContext context) {
    final macController = TextEditingController();
    return _card(title: 'Wake-on-LAN', children: [
      _textOption(
        context,
        label: 'Configured device MACs',
        key: 'q2-device-macs',
        hint: '100.64.0.2=AA:BB:CC:DD:EE:FF',
        helperText: 'Pairs of peer address and MAC address (comma-separated)',
      ),
      Row(
        children: [
          Text(
            '${translate("Target MAC")}: ',
            style: const TextStyle(fontSize: 14),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: TextField(
              controller: macController,
              decoration: const InputDecoration(
                hintText: 'AA:BB:CC:DD:EE:FF',
                helperText: 'Send magic packet direct or via online peer relay',
                contentPadding:
                    EdgeInsets.symmetric(vertical: 12, horizontal: 12),
              ),
            ).workaroundFreezeLinuxMint(),
          ),
          const SizedBox(width: 8),
          ElevatedButton(
            onPressed: () async {
              final target = macController.text.trim();
              if (target.isNotEmpty) {
                await bind.mainSetOption(
                  key: 'q2-wake-trigger',
                  value: target,
                );
                BotToast.showText(text: translate('Wake packet sent'));
              }
            },
            child: Text(translate('Wake')),
          ),
        ],
      ).marginOnly(left: _kContentHMargin, right: _kContentHMargin, top: 4),
    ]);
  }

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  Widget _boolOption(BuildContext context, String label, String key,
      {bool defaultValue = false}) {
    final raw = bind.mainGetOptionSync(key: key);
    bool value = raw.isEmpty ? defaultValue : (raw == 'Y');
    final ref = value.obs;

    onChanged(bool? option) async {
      if (option != null) {
        await bind.mainSetOption(key: key, value: option ? 'Y' : 'N');
        ref.value = option;
      }
    }

    return GestureDetector(
      child: Obx(
        () => Row(
          children: [
            Checkbox(value: ref.value, onChanged: onChanged)
                .marginOnly(right: 5),
            Expanded(child: Text(translate(label))),
          ],
        ),
      ).marginOnly(left: _kCheckBoxLeftMargin),
      onTap: () => onChanged(!ref.value),
    );
  }

  Widget _textOption(
    BuildContext context, {
    required String label,
    required String key,
    String hint = '',
    String? helperText,
  }) {
    final stored = bind.mainGetOptionSync(key: key);
    final controller = TextEditingController(text: stored);
    final typed = RxString(stored);
    final saved = RxString(stored);

    return Row(
      children: [
        Text(
          '${translate(label)}: ',
          style: const TextStyle(fontSize: 14),
        ),
        const SizedBox(width: 8),
        Expanded(
          child: TextField(
            controller: controller,
            onChanged: (v) => typed.value = v,
            decoration: InputDecoration(
              hintText: hint,
              helperText: helperText,
              contentPadding:
                  const EdgeInsets.symmetric(vertical: 12, horizontal: 12),
            ),
          ).workaroundFreezeLinuxMint(),
        ),
        const SizedBox(width: 8),
        Obx(() => Offstage(
              offstage: typed.value.trim() == saved.value.trim(),
              child: ElevatedButton(
                onPressed: () async {
                  final v = controller.text.trim();
                  await bind.mainSetOption(key: key, value: v);
                  typed.value = v;
                  saved.value = v;
                },
                child: Text(translate('Apply')),
              ),
            )),
      ],
    ).marginOnly(left: _kContentHMargin, right: _kContentHMargin);
  }
}

Widget _card({required String title, required List<Widget> children}) {
  return Row(
    children: [
      Flexible(
        child: SizedBox(
          width: _kCardFixedWidth,
          child: Card(
            child: Column(
              children: [
                Row(
                  children: [
                    Expanded(
                      child: Text(
                        translate(title),
                        textAlign: TextAlign.start,
                        style: const TextStyle(fontSize: _kTitleFontSize),
                      ),
                    ),
                  ],
                ).marginOnly(left: _kContentHMargin, top: 10, bottom: 10),
                ...children
                    .map((e) => e.marginOnly(top: 4, right: _kContentHMargin)),
              ],
            ).marginOnly(bottom: 10),
          ).marginOnly(left: _kCardLeftMargin, top: 15),
        ),
      ),
    ],
  );
}
