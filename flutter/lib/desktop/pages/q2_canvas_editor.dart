import 'dart:convert';
import 'dart:math';

import 'package:bot_toast/bot_toast.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/models/peer_model.dart';
import 'package:flutter_hbb/models/platform_model.dart';

/// Canvas Editor for physical monitor layout, snapping, and placements (R-UI-2, R-LAYOUT-1, R-LAYOUT-8, R-PLACE-5).
class Q2CanvasEditor extends StatefulWidget {
  const Q2CanvasEditor({Key? key}) : super(key: key);

  @override
  State<Q2CanvasEditor> createState() => Q2CanvasEditorState();
}

class _EditorMonitor {
  final String device;
  final int index;
  String name;
  double xMm;
  double yMm;
  double widthMm;
  double heightMm;
  int widthPx;
  int heightPx;

  _EditorMonitor({
    required this.device,
    required this.index,
    this.name = '',
    required this.xMm,
    required this.yMm,
    required this.widthMm,
    required this.heightMm,
    required this.widthPx,
    required this.heightPx,
  });

  String get id => '$device:$index';
}

class Q2CanvasEditorState extends State<Q2CanvasEditor> {
  final List<_EditorMonitor> _monitors = [];
  final Map<String, String> _placements = {};
  final Set<String> _activeDevices = {'local'};
  int _version = 0;
  String _activeProfile = 'Default';
  double _scale = 0.22; // widget px per real mm

  Set<String> get activeDevices => Set.unmodifiable(_activeDevices);

  @override
  void initState() {
    super.initState();
    _loadLayout();

    // Listen to real-time layout updates from peers over TCP port 21130
    platformFFI.registerEventHandler('q2_layout_updated', 'q2_canvas_editor', (evt) async {
      if (mounted) {
        _loadLayout();
      }
    });
  }

  @override
  void dispose() {
    platformFFI.unregisterEventHandler('q2_layout_updated', 'q2_canvas_editor');
    super.dispose();
  }

  void setDeviceSessionActive(String device, bool active) {
    setState(() {
      if (active) {
        _activeDevices.add(device);
        if (_placements[device] == 'Inactive' || _placements[device] == null) {
          _placements[device] = 'Beside';
        }
      } else {
        _activeDevices.remove(device);
        _placements[device] = 'Inactive';
      }
    });
    _saveAndSync();
  }

  void _loadLayout() {
    final Map<String, List<double>> savedOrigins = {};

    final raw = bind.mainGetOptionSync(key: 'q2-layout-doc');
    if (raw.isNotEmpty) {
      try {
        final parsed = jsonDecode(raw) as Map<String, dynamic>;
        _version = parsed['version'] as int? ?? 0;
        final data = parsed['data'] as Map<String, dynamic>? ?? {};
        final mons = data['monitors'] as List<dynamic>? ?? [];
        for (final m in mons) {
          final id = m['id'] as Map<String, dynamic>? ?? {};
          final dev = id['device'] as String? ?? 'unknown';
          final idx = id['local_index'] as int? ?? 0;
          final origin = m['origin_mm'] as List<dynamic>? ?? [0.0, 0.0];
          savedOrigins['$dev:$idx'] = [
            (origin[0] as num).toDouble(),
            (origin[1] as num).toDouble(),
          ];
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

    _monitors.clear();
    _activeDevices.clear();
    _activeDevices.add('local');

    // 1. Populate real local monitors
    final localRaw = bind.mainGetOptionSync(key: 'q2-local-panels');
    List<dynamic> localPanels = [];
    if (localRaw.isNotEmpty) {
      try {
        localPanels = jsonDecode(localRaw) as List<dynamic>? ?? [];
      } catch (_) {}
    }

    if (localPanels.isNotEmpty) {
      for (final p in localPanels) {
        final idx = (p['index'] as num?)?.toInt() ?? 0;
        final wPx = (p['width'] as num?)?.toInt() ?? 1920;
        final hPx = (p['height'] as num?)?.toInt() ?? 1080;
        double wMm = (p['width_mm'] as num?)?.toDouble() ?? 0.0;
        double hMm = (p['height_mm'] as num?)?.toDouble() ?? 0.0;
        if (wMm <= 0.0) wMm = wPx * 25.4 / 96.0;
        if (hMm <= 0.0) hMm = hPx * 25.4 / 96.0;

        final saved = savedOrigins['local:$idx'];
        final originX = saved != null ? saved[0] : (idx * (wMm + 8.0));
        final originY = saved != null ? saved[1] : 0.0;

        _monitors.add(_EditorMonitor(
          device: 'local',
          index: idx,
          name: 'Screen ${idx + 1}',
          xMm: originX,
          yMm: originY,
          widthMm: wMm,
          heightMm: hMm,
          widthPx: wPx,
          heightPx: hPx,
        ));
      }
    } else {
      // Fallback local monitor
      final saved = savedOrigins['local:0'];
      _monitors.add(_EditorMonitor(
        device: 'local',
        index: 0,
        name: 'Main Display',
        xMm: saved != null ? saved[0] : 0.0,
        yMm: saved != null ? saved[1] : 0.0,
        widthMm: 597.0,
        heightMm: 336.0,
        widthPx: 2560,
        heightPx: 1440,
      ));
    }

    // 2. Populate discovered peer monitors
    final lanPeers = gFFI.lanPeersModel.peers;
    double nextRightMm = _monitors.fold<double>(0.0, (prev, m) => max(prev, m.xMm + m.widthMm)) + 12.0;

    for (final peer in lanPeers) {
      final dev = peer.id;
      final currentPl = _placements[dev];
      if (currentPl != 'Inactive' && (currentPl != null || peer.online)) {
        _activeDevices.add(dev);
        if (_placements[dev] == null) {
          _placements[dev] = 'Beside';
        }
      }

      // Query real panels for peer
      final peerRaw = bind.mainGetOptionSync(key: 'q2-peer-monitors:$dev');
      List<dynamic> peerPanels = [];
      if (peerRaw.isNotEmpty) {
        try {
          peerPanels = jsonDecode(peerRaw) as List<dynamic>? ?? [];
        } catch (_) {}
      }

      final peerName = peer.hostname.isNotEmpty ? peer.hostname : (peer.alias.isNotEmpty ? peer.alias : dev);

      if (peerPanels.isNotEmpty) {
        for (final p in peerPanels) {
          final idx = (p['index'] as num?)?.toInt() ?? 0;
          final wPx = (p['width'] as num?)?.toInt() ?? 1920;
          final hPx = (p['height'] as num?)?.toInt() ?? 1080;
          double wMm = (p['width_mm'] as num?)?.toDouble() ?? 0.0;
          double hMm = (p['height_mm'] as num?)?.toDouble() ?? 0.0;
          if (wMm <= 0.0) wMm = wPx * 25.4 / 96.0;
          if (hMm <= 0.0) hMm = hPx * 25.4 / 96.0;

          final saved = savedOrigins['$dev:$idx'];
          final originX = saved != null ? saved[0] : nextRightMm;
          final originY = saved != null ? saved[1] : 0.0;
          if (saved == null) {
            nextRightMm += wMm + 12.0;
          }

          _monitors.add(_EditorMonitor(
            device: dev,
            index: idx,
            name: '$peerName ${idx + 1}',
            xMm: originX,
            yMm: originY,
            widthMm: wMm,
            heightMm: hMm,
            widthPx: wPx,
            heightPx: hPx,
          ));
        }
      } else {
        // Fallback peer display
        final saved = savedOrigins['$dev:0'];
        final originX = saved != null ? saved[0] : nextRightMm;
        final originY = saved != null ? saved[1] : 0.0;
        if (saved == null) {
          nextRightMm += 520.0 + 12.0;
        }

        _monitors.add(_EditorMonitor(
          device: dev,
          index: 0,
          name: peerName,
          xMm: originX,
          yMm: originY,
          widthMm: 520.0,
          heightMm: 293.0,
          widthPx: 1920,
          heightPx: 1080,
        ));
      }
    }

    _updateProfileIndicator();
    setState(() {});
  }

  void _updateProfileIndicator() {
    final devSet = _monitors.map((m) => m.device).toSet();
    _activeProfile = devSet.join(' + ');
  }

  void _checkBesideOrInside(_EditorMonitor moving) {
    if (moving.device == 'local') return;

    // Check overlap with any local monitor
    _EditorMonitor? host;
    double maxOverlap = 0.0;
    for (final other in _monitors) {
      if (other.device != 'local') continue;
      final x0 = max(moving.xMm, other.xMm);
      final x1 = min(moving.xMm + moving.widthMm, other.xMm + other.widthMm);
      final y0 = max(moving.yMm, other.yMm);
      final y1 = min(moving.yMm + moving.heightMm, other.yMm + other.heightMm);
      if (x1 > x0 && y1 > y0) {
        final area = (x1 - x0) * (y1 - y0);
        if (area > maxOverlap) {
          maxOverlap = area;
          host = other;
        }
      }
    }

    final movingArea = moving.widthMm * moving.heightMm;
    if (host != null && (maxOverlap / movingArea) > 0.40) {
      // Overlap detected: switch to Inside mode
      final current = _placements[moving.device] ?? 'Beside';
      if (!current.startsWith('Inside')) {
        _placements[moving.device] = 'Inside:Windowed';
      }
    } else {
      // Dragged out: revert to Beside
      if (_placements[moving.device]?.startsWith('Inside') == true) {
        _placements[moving.device] = 'Beside';
      }
      _snapMonitor(moving);
    }
  }

  void _snapMonitor(_EditorMonitor moving) {
    const double snapThresholdMm = 20.0;
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
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _buildToolbar(context),
        const Divider(height: 1),
        Expanded(child: _buildCanvas(context)),
      ],
    );
  }

  Widget _buildToolbar(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      color: Theme.of(context).cardColor,
      child: Row(
        children: [
          const Icon(Icons.dashboard_customize_outlined, size: 18, color: MyTheme.accent),
          const SizedBox(width: 8),
          Text(
            '${translate("Active Setup")}: $_activeProfile',
            style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 13),
          ),
          const Spacer(),
          IconButton(
            icon: const Icon(Icons.zoom_out, size: 18),
            tooltip: 'Zoom out',
            onPressed: () => setState(() => _scale = max(0.1, _scale - 0.04)),
          ),
          Text('${(_scale * 450).toInt()}%', style: const TextStyle(fontSize: 12)),
          IconButton(
            icon: const Icon(Icons.zoom_in, size: 18),
            tooltip: 'Zoom in',
            onPressed: () => setState(() => _scale = min(0.6, _scale + 0.04)),
          ),
          const SizedBox(width: 8),
          IconButton(
            icon: const Icon(Icons.filter_center_focus, size: 18),
            tooltip: 'Center View',
            onPressed: () {
              setState(() {
                _scale = 0.22;
              });
            },
          ),
          const SizedBox(width: 8),
          ElevatedButton.icon(
            icon: const Icon(Icons.sync, size: 15),
            label: Text(translate('Save & Broadcast')),
            style: ElevatedButton.styleFrom(padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8)),
            onPressed: () async {
              await _saveAndSync();
              BotToast.showText(text: translate('Layout synchronized to all devices'));
            },
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
            painter: _GridAndLinksPainter(
              monitors: _monitors.where((m) => _activeDevices.contains(m.device)).toList(),
              scale: _scale,
            ),
          ),
          ..._monitors.map((m) => _buildDraggableMonitor(m)),
        ],
      ),
    );
  }

  Widget _buildDraggableMonitor(_EditorMonitor m) {
    final left = max(10.0, 40.0 + m.xMm * _scale);
    final top = max(10.0, 40.0 + m.yMm * _scale);
    final width = max(80.0, m.widthMm * _scale);
    final height = max(55.0, m.heightMm * _scale);

    final isLocal = m.device == 'local';
    final placement = _placements[m.device] ?? 'Beside';
    final isActive = _activeDevices.contains(m.device);

    // Look up peer if not local
    Peer? peer;
    if (!isLocal) {
      final peers = gFFI.lanPeersModel.peers;
      peer = peers.firstWhere((p) => p.id == m.device, orElse: () => Peer.fromJson({'id': m.device}));
    }
    final isOnline = isLocal || (peer?.online ?? false);

    // Color coding based on user specification:
    // 1. Grayscale: Sleeping / Offline
    // 2. Blue / Neutral: Available (online, but not active in session)
    // 3. Green / Accent: Active in session (Beside)
    // 4. Purple / Indigo: Active in session (Inside / Remote Desktop)
    Color borderColor;
    Color fillColor;
    String modeBadge;

    if (!isOnline) {
      borderColor = Colors.grey.shade400;
      fillColor = Colors.grey.withOpacity(0.08);
      modeBadge = 'Sleeping (WoL)';
    } else if (!isActive) {
      borderColor = Colors.blue.shade400;
      fillColor = Colors.blue.withOpacity(0.06);
      modeBadge = 'Available (Inactive)';
    } else if (placement.startsWith('Inside')) {
      borderColor = Colors.purple.shade400;
      fillColor = Colors.purple.withOpacity(0.12);
      modeBadge = 'Inside';
    } else {
      borderColor = isLocal ? MyTheme.accent : Colors.green.shade600;
      fillColor = isLocal
          ? Theme.of(context).colorScheme.primary.withOpacity(0.14)
          : Colors.green.withOpacity(0.12);
      modeBadge = isLocal ? 'Host' : 'Beside (KVM)';
    }

    return Positioned(
      left: left,
      top: top,
      child: GestureDetector(
        onPanUpdate: (details) {
          setState(() {
            m.xMm += details.delta.dx / _scale;
            m.yMm += details.delta.dy / _scale;
            _checkBesideOrInside(m);
          });
        },
        onPanEnd: (_) {
          _saveAndSync();
        },
        child: Container(
          width: width,
          height: height,
          decoration: BoxDecoration(
            color: fillColor,
            border: Border.all(
              color: borderColor,
              width: isActive ? 2.2 : 1.2,
            ),
            borderRadius: BorderRadius.circular(8),
            boxShadow: [
              if (isActive)
                BoxShadow(
                  color: borderColor.withOpacity(0.18),
                  blurRadius: 6,
                  offset: const Offset(0, 2),
                ),
            ],
          ),
          padding: const EdgeInsets.all(6),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Expanded(
                    child: Text(
                      m.name.isNotEmpty ? m.name : '${m.device} [${m.index}]',
                      style: TextStyle(
                        fontWeight: FontWeight.bold,
                        fontSize: 11,
                        color: isOnline ? Theme.of(context).textTheme.titleSmall?.color : Colors.grey,
                      ),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                    decoration: BoxDecoration(
                      color: borderColor.withOpacity(0.2),
                      borderRadius: BorderRadius.circular(3),
                    ),
                    child: Text(
                      modeBadge,
                      style: TextStyle(fontSize: 9, color: borderColor, fontWeight: FontWeight.bold),
                    ),
                  ),
                ],
              ),
              const Spacer(),
              Text(
                '${m.widthMm.toInt()}×${m.heightMm.toInt()} mm (${m.widthPx}×${m.heightPx})',
                style: const TextStyle(fontSize: 9, color: Colors.grey),
                overflow: TextOverflow.ellipsis,
              ),
              if (!isLocal && isActive && placement.startsWith('Inside')) ...[
                const SizedBox(height: 3),
                _buildInsideModeSelector(m.device, placement),
              ],
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildInsideModeSelector(String dev, String currentPlacement) {
    final isFullscreen = currentPlacement == 'Inside:Fullscreen';

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 2, vertical: 1),
      decoration: BoxDecoration(
        color: Colors.purple.withOpacity(0.2),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          InkWell(
            onTap: () {
              setState(() {
                _placements[dev] = 'Inside:Windowed';
              });
              _saveAndSync();
            },
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
              decoration: BoxDecoration(
                color: !isFullscreen ? Colors.purple.shade600 : Colors.transparent,
                borderRadius: BorderRadius.circular(3),
              ),
              child: Text(
                'Window',
                style: TextStyle(
                  fontSize: 9,
                  color: !isFullscreen ? Colors.white : Colors.purple.shade100,
                  fontWeight: FontWeight.bold,
                ),
              ),
            ),
          ),
          const SizedBox(width: 2),
          InkWell(
            onTap: () {
              setState(() {
                _placements[dev] = 'Inside:Fullscreen';
              });
              _saveAndSync();
            },
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
              decoration: BoxDecoration(
                color: isFullscreen ? Colors.purple.shade600 : Colors.transparent,
                borderRadius: BorderRadius.circular(3),
              ),
              child: Text(
                'Fullscreen',
                style: TextStyle(
                  fontSize: 9,
                  color: isFullscreen ? Colors.white : Colors.purple.shade100,
                  fontWeight: FontWeight.bold,
                ),
              ),
            ),
          ),
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

    for (double x = 0; x < size.width; x += 30) {
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), paintGrid);
    }
    for (double y = 0; y < size.height; y += 30) {
      canvas.drawLine(Offset(0, y), Offset(size.width, y), paintGrid);
    }

    final paintLink = Paint()
      ..color = Colors.green.shade500
      ..strokeWidth = 3.5
      ..strokeCap = StrokeCap.round
      ..style = PaintingStyle.stroke;

    // Draw active Beside boundary links between adjacent monitors
    for (int i = 0; i < monitors.length; i++) {
      for (int j = i + 1; j < monitors.length; j++) {
        final a = monitors[i];
        final b = monitors[j];
        final ax = 40.0 + a.xMm * scale;
        final ay = 40.0 + a.yMm * scale;
        final aw = a.widthMm * scale;
        final ah = a.heightMm * scale;

        final bx = 40.0 + b.xMm * scale;
        final by = 40.0 + b.yMm * scale;
        final bw = b.widthMm * scale;
        final bh = b.heightMm * scale;

        // Check horizontal adjacency (Beside crossing boundary)
        if ((bx - (ax + aw)).abs() < 16 * scale || (ax - (bx + bw)).abs() < 16 * scale) {
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
