import 'package:bot_toast/bot_toast.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/desktop/pages/desktop_tab_page.dart';
import 'package:flutter_hbb/desktop/pages/q2_canvas_editor.dart';
import 'package:flutter_hbb/models/peer_model.dart';
import 'package:flutter_hbb/models/platform_model.dart';

/// The primary Home Workspace Hub for q2crazyDesk.
/// Replaces the legacy RustDesk connection view with a clean canvas
/// and color-coded device session controls.
class Q2WorkspacePage extends StatefulWidget {
  const Q2WorkspacePage({Key? key}) : super(key: key);

  @override
  State<Q2WorkspacePage> createState() => _Q2WorkspacePageState();
}

class _Q2WorkspacePageState extends State<Q2WorkspacePage> {
  final GlobalKey<Q2CanvasEditorState> _canvasKey = GlobalKey<Q2CanvasEditorState>();

  @override
  void initState() {
    super.initState();
    gFFI.lanPeersModel.addListener(_onPeersChanged);
  }

  @override
  void dispose() {
    gFFI.lanPeersModel.removeListener(_onPeersChanged);
    super.dispose();
  }

  void _onPeersChanged() {
    if (mounted) {
      setState(() {});
    }
  }

  void _wakePeer(Peer peer) {
    // Try to retrieve MAC address from peer info or options
    String mac = '';
    final storedMac = bind.mainGetOptionSync(key: 'q2-wol-mac-${peer.id}');
    if (storedMac.isNotEmpty) {
      mac = storedMac;
    }
    if (mac.isEmpty) {
      // Prompt user or toast
      BotToast.showText(
        text: translate('No MAC address configured for ${peer.hostname}. Setting wake trigger...'),
      );
      bind.mainSetOption(key: 'q2-wake-trigger', value: peer.id);
      return;
    }

    final res = bind.mainGetOptionSync(key: 'q2-wake-peer:$mac');
    if (res == 'OK') {
      BotToast.showText(text: translate('Wake packet sent to ${peer.hostname}'));
    } else {
      BotToast.showText(text: translate('Failed to send wake packet'));
    }
  }

  void _togglePeerSession(Peer peer, bool active) {
    // Ensure peer IP is in paired list if activating
    if (active) {
      final raw = bind.mainGetOptionSync(key: 'q2-paired');
      final ids = raw
          .split(',')
          .map((s) => s.trim())
          .where((s) => s.isNotEmpty)
          .toSet();
      ids.add(peer.id);
      final sorted = ids.toList()..sort();
      bind.mainSetOption(key: 'q2-paired', value: sorted.join(','));
    }

    _canvasKey.currentState?.setDeviceSessionActive(peer.id, active);
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Theme.of(context).scaffoldBackgroundColor,
      body: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _buildDeviceControlBar(context),
          const Divider(height: 1),
          Expanded(
            child: Q2CanvasEditor(key: _canvasKey),
          ),
        ],
      ),
    );
  }

  Widget _buildDeviceControlBar(BuildContext context) {
    final lanPeers = gFFI.lanPeersModel.peers;
    final activeDevices = _canvasKey.currentState?.activeDevices ?? <String>{'local'};

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
      color: Theme.of(context).cardColor,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              const Icon(Icons.hub_outlined, size: 22, color: MyTheme.accent),
              const SizedBox(width: 8),
              const Text(
                'q2crazyDesk Workspace',
                style: TextStyle(fontWeight: FontWeight.bold, fontSize: 16),
              ),
              const SizedBox(width: 14),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                decoration: BoxDecoration(
                  color: Colors.green.withOpacity(0.15),
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: Colors.green.shade600, width: 1),
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Container(
                      width: 8,
                      height: 8,
                      decoration: const BoxDecoration(
                        color: Colors.green,
                        shape: BoxShape.circle,
                      ),
                    ),
                    const SizedBox(width: 6),
                    Text(
                      'Tailnet: ${lanPeers.where((p) => p.online).length + 1} online',
                      style: TextStyle(fontSize: 11, color: Colors.green.shade800, fontWeight: FontWeight.bold),
                    ),
                  ],
                ),
              ),
              const Spacer(),
              IconButton(
                icon: const Icon(Icons.settings_outlined, size: 20),
                tooltip: translate('Settings'),
                onPressed: () => DesktopTabPage.onAddSetting(),
              ),
            ],
          ),
          const SizedBox(height: 10),
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: [
                _buildLocalDeviceCard(context),
                ...lanPeers.map((peer) {
                  final isActive = activeDevices.contains(peer.id);
                  return _buildPeerDeviceCard(context, peer, isActive);
                }),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildLocalDeviceCard(BuildContext context) {
    return Container(
      margin: const EdgeInsets.only(right: 12),
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.primary.withOpacity(0.08),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: MyTheme.accent, width: 1.5),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 10,
            height: 10,
            decoration: const BoxDecoration(color: Colors.green, shape: BoxShape.circle),
          ),
          const SizedBox(width: 8),
          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                '${translate("This PC")} (local)',
                style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 13),
              ),
              const Text(
                'Host / Controller',
                style: TextStyle(fontSize: 10, color: Colors.grey),
              ),
            ],
          ),
          const SizedBox(width: 12),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
            decoration: BoxDecoration(
              color: Colors.green.withOpacity(0.2),
              borderRadius: BorderRadius.circular(4),
            ),
            child: const Text('Active', style: TextStyle(fontSize: 10, color: Colors.green, fontWeight: FontWeight.bold)),
          ),
        ],
      ),
    );
  }

  Widget _buildPeerDeviceCard(BuildContext context, Peer peer, bool isActive) {
    final isOnline = peer.online;
    
    // Determine Color coding:
    // Green: Online & Active in session
    // Blue: Online & Available (not in session)
    // Gray: Sleeping / Offline (can wake)
    Color statusColor;
    String statusLabel;
    if (isOnline && isActive) {
      statusColor = Colors.green;
      statusLabel = 'Active in session';
    } else if (isOnline && !isActive) {
      statusColor = Colors.blue;
      statusLabel = 'Available';
    } else {
      statusColor = Colors.grey;
      statusLabel = 'Sleeping';
    }

    final displayName = peer.hostname.isNotEmpty ? peer.hostname : (peer.alias.isNotEmpty ? peer.alias : peer.id);

    return Container(
      margin: const EdgeInsets.only(right: 12),
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: isOnline
            ? (isActive ? Colors.green.withOpacity(0.06) : Colors.blue.withOpacity(0.06))
            : Colors.grey.withOpacity(0.06),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: isOnline
              ? (isActive ? Colors.green.shade600 : Colors.blue.shade400)
              : Colors.grey.shade400,
          width: isActive ? 1.8 : 1.0,
        ),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 10,
            height: 10,
            decoration: BoxDecoration(color: statusColor, shape: BoxShape.circle),
          ),
          const SizedBox(width: 8),
          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(
                    peer.platform.toLowerCase().contains('mac') ? Icons.apple : Icons.desktop_windows,
                    size: 14,
                    color: Colors.grey.shade600,
                  ),
                  const SizedBox(width: 4),
                  Text(
                    displayName,
                    style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 13),
                  ),
                ],
              ),
              Text(
                '${peer.id} • $statusLabel',
                style: TextStyle(fontSize: 10, color: statusColor),
              ),
            ],
          ),
          const SizedBox(width: 14),
          if (isOnline)
            Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  isActive ? 'Connected' : 'Connect',
                  style: TextStyle(fontSize: 11, color: statusColor, fontWeight: FontWeight.bold),
                ),
                Switch(
                  value: isActive,
                  activeColor: Colors.green,
                  onChanged: (val) => _togglePeerSession(peer, val),
                ),
              ],
            )
          else
            TextButton.icon(
              icon: const Icon(Icons.alarm, size: 14),
              label: const Text('Wake', style: TextStyle(fontSize: 11)),
              onPressed: () => _wakePeer(peer),
              style: TextButton.styleFrom(
                foregroundColor: Colors.grey.shade700,
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                minimumSize: Size.zero,
                tapTargetSize: MaterialTapTargetSize.shrinkWrap,
              ),
            ),
        ],
      ),
    );
  }
}
