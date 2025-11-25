import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:provider/provider.dart';
import 'package:restsend_dart/restsend_dart.dart';

import 'chat_screen.dart';
import 'demo_controller.dart';

class ConversationListScreen extends StatelessWidget {
  const ConversationListScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final controller = context.watch<DemoController>();
    final conversations = controller.conversations;
    final syncProgress = controller.syncProgress;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Conversations'),
        actions: [
          IconButton(
            tooltip: 'Reload conversations',
            icon: const Icon(Icons.refresh),
            onPressed: controller.isLoadingConversations
                ? null
                : () => controller.loadConversations(refresh: true),
          ),
          IconButton(
            tooltip: 'Sync from server',
            icon: const Icon(Icons.sync),
            onPressed: controller.isSyncingConversations
                ? null
                : () => controller.syncConversations(),
          ),
          IconButton(
            tooltip: 'Logout',
            icon: const Icon(Icons.logout),
            onPressed:
                controller.isAuthenticated ? () => controller.logout() : null,
          ),
        ],
      ),
      body: Column(
        children: [
          if (syncProgress != null)
            _SyncBanner(
              progress: syncProgress,
              isActive: controller.isSyncingConversations,
            ),
          if (controller.errorMessage != null)
            Padding(
              padding: const EdgeInsets.all(8),
              child: Text(
                controller.errorMessage!,
                style: TextStyle(
                  color: Theme.of(context).colorScheme.error,
                ),
              ),
            ),
          Expanded(
            child: RefreshIndicator(
              onRefresh: () => controller.loadConversations(refresh: true),
              child: conversations.isEmpty
                  ? ListView(
                      children: const [
                        SizedBox(height: 120),
                        Center(child: Text('No conversations yet.')),
                      ],
                    )
                  : ListView.separated(
                      padding: const EdgeInsets.symmetric(vertical: 8),
                      itemCount: conversations.length,
                      separatorBuilder: (_, __) => const Divider(height: 1),
                      itemBuilder: (context, index) {
                        final conversation = conversations[index];
                        return _ConversationTile(conversation: conversation);
                      },
                    ),
            ),
          ),
        ],
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: controller.isSyncingConversations
            ? null
            : () => controller.syncConversations(),
        icon: const Icon(Icons.sync),
        label: Text(
          controller.isSyncingConversations ? 'Syncing…' : 'Sync conversations',
        ),
      ),
    );
  }
}

class _ConversationTile extends StatelessWidget {
  const _ConversationTile({required this.conversation});

  final RestsendConversation conversation;

  @override
  Widget build(BuildContext context) {
    final unreadCount = conversation.unread.toInt();
    final lastMessagePreview = _buildPreview(conversation.lastMessage);
    final updatedAt = _formatTime(conversation.lastMessageAt);
    return ListTile(
      onTap: () {
        Navigator.of(context).push(
          MaterialPageRoute(
            builder: (_) => ChatScreen(conversation: conversation),
          ),
        );
      },
      leading: CircleAvatar(
        child: Text(conversation.name.isEmpty
            ? conversation.topicId.characters.take(2).toString().toUpperCase()
            : conversation.name.characters.take(2).toString().toUpperCase()),
      ),
      title: Text(conversation.name.isEmpty
          ? 'Conversation ${conversation.topicId}'
          : conversation.name),
      subtitle: Text('$lastMessagePreview • $updatedAt'),
      trailing: unreadCount > 0
          ? CircleAvatar(
              radius: 12,
              backgroundColor: Theme.of(context).colorScheme.primary,
              foregroundColor: Theme.of(context).colorScheme.onPrimary,
              child: Text(
                unreadCount.toString(),
                style: const TextStyle(fontSize: 12),
              ),
            )
          : null,
    );
  }

  String _buildPreview(RestsendContent? content) {
    if (content == null) {
      return 'No messages yet';
    }
    if (content.text.isNotEmpty) {
      return content.text;
    }
    if (content.placeholder.isNotEmpty) {
      return content.placeholder;
    }
    if (content.attachment != null) {
      return 'Attachment: ${content.attachment!.fileName}';
    }
    return 'Unsupported message type';
  }

  String _formatTime(String source) {
    if (source.isEmpty) {
      return '-';
    }
    final parsed = DateTime.tryParse(source);
    if (parsed == null) {
      return source;
    }
    return DateFormat('MM-dd HH:mm').format(parsed);
  }
}

class _SyncBanner extends StatelessWidget {
  const _SyncBanner({required this.progress, required this.isActive});

  final SyncProgress progress;
  final bool isActive;

  @override
  Widget build(BuildContext context) {
    final percent = progress.total == 0
        ? 0.0
        : (progress.count / progress.total).clamp(0, 1).toDouble();
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Expanded(
                child: LinearProgressIndicator(
                  value: progress.total == 0 ? null : percent,
                ),
              ),
              const SizedBox(width: 12),
              Text('${progress.count}/${progress.total}')
            ],
          ),
          const SizedBox(height: 4),
          Text(
            isActive
                ? 'Syncing… last updated ${progress.updatedAt}'
                : 'Sync complete',
          ),
        ],
      ),
    );
  }
}
