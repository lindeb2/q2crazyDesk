import 'dart:convert';
import 'dart:math';

import 'package:bot_toast/bot_toast.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/models/platform_model.dart';

/// Canvas Editor for physical monitor layout, snapping, and placements (R-UI-2, R-LAYOUT-1, R-LAYOUT-8, R-PLACE-5).
class Q2CanvasEditor extends StatefulWidget {
  const Q2CanvasEditor({Key? key}) : super(key: key);

  @override
  State<Q2CanvasEditor> createState() => _Q2CanvasEditorState();
}

class _EditorMonitor {
  final String device;
  final int index;
  double xMm;
  double yMm;
  double widthMm;
  double heightMm;
  int widthPx;
  int heightPx;

  _EditorMonitor({
    required this.device,
    required this.index,
    required this.xMm,
    required this.yMm,
    required this.widthMm,
    required this.heightMm,
    required this.widthPx,
    required this.heightPx,
  });

  String get id => '$device:$index';
}

class _Q2CanvasEditorState extends State<Q2CanvasEditor> {
  final List<_EditorMonitor> _monitors = [];
  final Map<String, String> _placements = {};
  int _version = 0;
  String _activeProfile = 'Default';
  double _scale = 0.25; // 0.25 widget px per real mm

  @override
  void initState() {
    super.initState();
    _loadLayout();
  }

  void _loadLayout() {
    final raw = bind.mainGetOptionSync(key: 'q2-layout-doc');
    if (raw.isNotEmpty) {
      try {
        final parsed = jsonDecode(raw) as Map<String, dynamic>;
        _version = parsed['version'] as int? ?? 0;
        final data = parsed['data'] as Map<String, dynamic>? ?? {};
        final mons = data['monitors'] as List<dynamic>? ?? [];
        _monitors.clear();
        for (final m in mons) {
          final id = m['id'] as Map<String, dynamic>? ?? {};
          final dev = id['device'] as String? ?? 'unknown';
          final idx = id['local_index'] as int? ?? 0;
          final origin = m['origin_mm'] as List<dynamic>? ?? [0.0, 0.0];
          final phys = m['physical_mm'] as Map<String, dynamic>? ?? {};
          final px = m['pixels'] as Map<String, dynamic>? ?? {};
          _monitors.add(_EditorMonitor(
            device: dev,
            index: idx,
            xMm: (origin[0] as num).toDouble(),
            yMm: (origin[1] as num).toDouble(),
            widthMm: (phys['width'] as num?)?.toDouble() ?? 597.0,
            heightMm: (phys['height'] as num?)?.toDouble() ?? 336.0,
            widthPx: (px['width'] as int?) ?? 1920,
            heightPx: (px['height'] as int?) ?? 1080,
          ));
        }

        final pls = data['placements'] as Map<String, dynamic>? ?? {};
        _placements.clear();
        pls.forEach((k, v) {
          if (v is String) {
            _placements[k] = v;
          } else if (v is Map) {
            _placements[k] = v.keys.first.toString();
          }
        });
      } catch (e) {
        debugPrint('Failed to parse q2-layout-doc: $e');
      }
    }

    if (_monitors.isEmpty) {
      // Provide initial default displays
      _monitors.add(_EditorMonitor(
        device: 'local',
        index: 0,
        xMm: 0,
        yMm: 0,
        widthMm: 597.0,
        heightMm: 336.0,
        widthPx: 2560,
        heightPx: 1440,
      ));
      _monitors.add(_EditorMonitor(
        device: 'peer',
        index: 0,
        xMm: 605.0,
        yMm: 20.0,
        widthMm: 521.0,
        heightMm: 293.0,
        widthPx: 1920,
        heightPx: 1080,
      ));
      _placements['peer'] = 'Beside';
    }

    _updateProfileIndicator();
  }

  void _updateProfileIndicator() {
    final devSet = _monitors.map((m) => m.device).toSet();
    setState(() {
      _activeProfile = devSet.join(' + ');
    });
  }

  void _snapMonitor(_EditorMonitor moving) {
    const double snapThresholdMm = 15.0;
    const double gapMm = 8.0;

    for (final other in _monitors) {
      if (other == moving) continue;

      // Snap moving left to other right
      final movingLeftToOtherRight = (moving.xMm - (other.xMm + other.widthMm + gapMm)).abs();
      if (movingLeftToOtherRight < snapThresholdMm) {
        moving.xMm = other.xMm + other.widthMm + gapMm;
      }

      // Snap moving right to other left
      final movingRightToOtherLeft = ((moving.xMm + moving.widthMm + gapMm) - other.xMm).abs();
      if (movingRightToOtherLeft < snapThresholdMm) {
        moving.xMm = other.xMm - moving.widthMm - gapMm;
      }

      // Snap Y alignment
      if ((moving.yMm - other.yMm).abs() < snapThresholdMm) {
        moving.yMm = other.yMm;
      }
    }
  }

  Future<void> _saveAndSync() async {
    _version += 1;
    final layoutMap = {
      'writer': 'local',
      'version': _version,
      'data': {
        'monitors': _monitors
            .map((m) => {
                  'id': {'device': m.device, 'local_index': m.index},
                  'origin_mm': [m.xMm, m.yMm],
                  'physical_mm': {'width': m.widthMm, 'height': m.heightMm},
                  'pixels': {'width': m.widthPx, 'height': m.heightPx},
                  'scale': 1.0,
                  'override_physical_mm': null,
                })
            .toList(),
        'placements': _placements,
        'links': [],
        'version': _version,
      }
    };

    final rawJson = jsonEncode(layoutMap);
    await bind.mainSetOption(key: 'q2-layout-doc', value: rawJson);
    BotToast.showText(text: translate('Layout saved and synced across devices'));
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _buildToolbar(context),
        const Divider(height: 1),
        Expanded(child: _buildCanvas(context)),
        const Divider(height: 1),
        _buildPlacementFooter(context),
      ],
    );
  }

  Widget _buildToolbar(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      color: Theme.of(context).cardColor,
      child: Row(
        children: [
          Icon(Icons.dashboard_customize_outlined, size: 20, color: MyTheme.accent),
          const SizedBox(width: 8),
          Text(
            '${translate("Active Profile")}: $_activeProfile',
            style: const TextStyle(fontWeight: FontWeight.bold),
          ),
          const Spacer(),
          IconButton(
            icon: const Icon(Icons.zoom_out),
            tooltip: 'Zoom out',
            onPressed: () => setState(() => _scale = max(0.1, _scale - 0.05)),
          ),
          Text('${(_scale * 400).toInt()}%'),
          IconButton(
            icon: const Icon(Icons.zoom_in),
            tooltip: 'Zoom in',
            onPressed: () => setState(() => _scale = min(0.6, _scale + 0.05)),
          ),
          const SizedBox(width: 12),
          ElevatedButton.icon(
            icon: const Icon(Icons.sync, size: 16),
            label: Text(translate('Save & Sync')),
            onPressed: _saveAndSync,
          ),
        ],
      ),
    );
  }

  Widget _buildCanvas(BuildContext context) {
    return Container(
      color: Theme.of(context).scaffoldBackgroundColor,
      child: Stack(
        children: [
          CustomPaint(
            size: Size.infinite,
            painter: _GridAndLinksPainter(monitors: _monitors, scale: _scale),
          ),
          ..._monitors.map((m) => _buildDraggableMonitor(m)),
        ],
      ),
    );
  }

  Widget _buildDraggableMonitor(_EditorMonitor m) {
    final left = max(10.0, 50.0 + m.xMm * _scale);
    final top = max(10.0, 50.0 + m.yMm * _scale);
    final width = max(60.0, m.widthMm * _scale);
    final height = max(40.0, m.heightMm * _scale);

    final placement = _placements[m.device] ?? 'Beside';
    final isLocal = m.device == 'local';

    return Positioned(
      left: left,
      top: top,
      child: GestureDetector(
        onPanUpdate: (details) {
          setState(() {
            m.xMm += details.delta.dx / _scale;
            m.yMm += details.delta.dy / _scale;
            _snapMonitor(m);
          });
        },
        child: Container(
          width: width,
          height: height,
          decoration: BoxDecoration(
            color: isLocal
                ? Theme.of(context).colorScheme.primary.withOpacity(0.18)
                : (placement.contains('Inside')
                    ? Colors.teal.withOpacity(0.2)
                    : Theme.of(context).colorScheme.secondary.withOpacity(0.18)),
            border: Border.all(
              color: isLocal ? MyTheme.accent : Colors.teal,
              width: 2,
            ),
            borderRadius: BorderRadius.circular(6),
          ),
          padding: const EdgeInsets.all(6),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                '${m.device} [${m.index}]',
                style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 12),
                overflow: TextOverflow.ellipsis,
              ),
              Text(
                '${m.widthMm.toInt()}×${m.heightMm.toInt()} mm (${m.widthPx}×${m.heightPx})',
                style: const TextStyle(fontSize: 10, color: Colors.grey),
                overflow: TextOverflow.ellipsis,
              ),
              if (!isLocal)
                Text(
                  'Mode: $placement',
                  style: const TextStyle(fontSize: 10, color: Colors.teal),
                ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildPlacementFooter(BuildContext context) {
    final peerDevices = _monitors
        .map((m) => m.device)
        .where((d) => d != 'local')
        .toSet()
        .toList();

    if (peerDevices.isEmpty) {
      return const SizedBox.shrink();
    }

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      color: Theme.of(context).cardColor,
      child: Row(
        children: [
          Text(
            '${translate("Quick placement")}: ',
            style: const TextStyle(fontWeight: FontWeight.bold),
          ),
          ...peerDevices.map((dev) {
            final current = _placements[dev] ?? 'Beside';
            return Row(
              children: [
                Text('$dev: '),
                DropdownButton<String>(
                  value: current,
                  items: const [
                    DropdownMenuItem(value: 'Beside', child: Text('Beside')),
                    DropdownMenuItem(value: 'Inside-Windowed', child: Text('Inside (Window)')),
                    DropdownMenuItem(value: 'Inside-Fullscreen', child: Text('Inside (Fullscreen)')),
                    DropdownMenuItem(value: 'Lent', child: Text('Lent')),
                  ],
                  onChanged: (val) {
                    if (val != null) {
                      setState(() {
                        _placements[dev] = val;
                      });
                      bind.mainSetOption(
                        key: 'q2-placement-$dev',
                        value: val.toLowerCase(),
                      );
                    }
                  },
                ),
                const SizedBox(width: 16),
              ],
            );
          }),
        ],
      ),
    );
  }
}

class _GridAndLinksPainter extends CustomPainter {
  final List<_EditorMonitor> monitors;
  final double scale;

  _GridAndLinksPainter({required this.monitors, required this.scale});

  @override
  void paint(Canvas canvas, Size size) {
    final paintGrid = Paint()
      ..color = Colors.grey.withOpacity(0.08)
      ..strokeWidth = 1;

    for (double x = 0; x < size.width; x += 25) {
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), paintGrid);
    }
    for (double y = 0; y < size.height; y += 25) {
      canvas.drawLine(Offset(0, y), Offset(size.width, y), paintGrid);
    }

    final paintLink = Paint()
      ..color = Colors.greenAccent
      ..strokeWidth = 3
      ..style = PaintingStyle.stroke;

    // Draw edge links when monitors are adjacent
    for (int i = 0; i < monitors.length; i++) {
      for (int j = i + 1; j < monitors.length; j++) {
        final a = monitors[i];
        final b = monitors[j];
        final ax = 50.0 + a.xMm * scale;
        final ay = 50.0 + a.yMm * scale;
        final aw = a.widthMm * scale;
        final ah = a.heightMm * scale;

        final bx = 50.0 + b.xMm * scale;
        final by = 50.0 + b.yMm * scale;
        final bw = b.widthMm * scale;
        final bh = b.heightMm * scale;

        // Check horizontal adjacency
        if ((bx - (ax + aw)).abs() < 12 * scale || (ax - (bx + bw)).abs() < 12 * scale) {
          final top = max(ay, by);
          final bottom = min(ay + ah, by + bh);
          if (bottom > top) {
            final x = (bx >= ax) ? ax + aw : bx + bw;
            canvas.drawLine(Offset(x, top), Offset(x, bottom), paintLink);
          }
        }
      }
    }
  }

  @override
  bool shouldRepaint(covariant _GridAndLinksPainter oldDelegate) => true;
}
