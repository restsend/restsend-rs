import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:restsend_dart/restsend_dart.dart';

import 'demo_controller.dart';

class ChatScreen extends StatefulWidget {
  const ChatScreen({super.key, required this.conversation});

  final RestsendConversation conversation;

  @override
  State<ChatScreen> createState() => _ChatScreenState();
}

class _ChatScreenState extends State<ChatScreen> {
  final _textController = TextEditingController();
  final _scrollController = ScrollController();
  final List<RestsendChatLog> _messages = [];

  bool _isLoading = true;
  bool _isSyncing = false;
  bool _isSending = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _loadLocal();
  }

  Future<void> _loadLocal() async {
    final controller = context.read<DemoController>();
    final client = controller.client;
    if (client == null) {
      setState(() {
        _isLoading = false;
        _error = 'Client is not connected.';
      });
      return;
    }
    try {
      final page = await client.getChatLogsLocal(
        topicId: widget.conversation.topicId,
        startSeq: 0,
        limit: 50,
      );
      _updateMessages(page.items);
    } catch (err) {
      setState(() {
        _error = 'Failed to load chat logs: $err';
      });
    } finally {
      if (mounted) {
        setState(() {
          _isLoading = false;
        });
      }
    }
  }

  Future<void> _syncRemote() async {
    final controller = context.read<DemoController>();
    final client = controller.client;
    if (client == null) {
      return;
    }
    setState(() {
      _isSyncing = true;
    });
    try {
      final lastSeq = _messages.isEmpty ? null : _messages.last.seq.toInt();
      final page = await client.syncChatLogs(
        topicId: widget.conversation.topicId,
        lastSeq: lastSeq,
        limit: 50,
        heavy: false,
      );
      _updateMessages(page.items);
      _scrollToBottom();
    } catch (err) {
      setState(() {
        _error = 'Failed to sync chat logs: $err';
      });
    } finally {
      if (mounted) {
        setState(() {
          _isSyncing = false;
        });
      }
    }
  }

  Future<void> _sendMessage() async {
    final text = _textController.text.trim();
    if (text.isEmpty) {
      return;
    }
    final controller = context.read<DemoController>();
    final client = controller.client;
    if (client == null) {
      return;
    }
    setState(() {
      _isSending = true;
    });
    try {
      await client.sendTextMessage(widget.conversation.topicId, text);
      _textController.clear();
      await _syncRemote();
    } catch (err) {
      setState(() {
        _error = 'Failed to send message: $err';
      });
    } finally {
      if (mounted) {
        setState(() {
          _isSending = false;
        });
      }
    }
  }

  void _updateMessages(List<RestsendChatLog> incoming) {
    if (!mounted) {
      return;
    }
    setState(() {
      final map = <String, RestsendChatLog>{
        for (final message in _messages) message.id: message,
      };
      for (final message in incoming) {
        map[message.id] = message;
      }
      final sorted = map.values.toList()
        ..sort((a, b) => a.seq.toInt().compareTo(b.seq.toInt()));
      _messages
        ..clear()
        ..addAll(sorted);
    });
  }

  void _scrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!_scrollController.hasClients) return;
      _scrollController.animateTo(
        _scrollController.position.maxScrollExtent,
        duration: const Duration(milliseconds: 300),
        curve: Curves.easeOut,
      );
    });
  }

  @override
  void dispose() {
    _textController.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final controller = context.watch<DemoController>();
    final currentUserId = controller.currentUserId;
    return Scaffold(
      appBar: AppBar(
        title: Text(widget.conversation.name.isEmpty
            ? widget.conversation.topicId
            : widget.conversation.name),
        actions: [
          IconButton(
            tooltip: 'Sync latest messages',
            icon: const Icon(Icons.sync),
            onPressed: _isSyncing ? null : _syncRemote,
          ),
        ],
      ),
      body: Column(
        children: [
          if (_error != null)
            Padding(
              padding: const EdgeInsets.all(8),
              child: Text(
                _error!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ),
          Expanded(
            child: _isLoading
                ? const Center(child: CircularProgressIndicator())
                : RefreshIndicator(
                    onRefresh: _syncRemote,
                    child: ListView.builder(
                      controller: _scrollController,
                      padding: const EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 16,
                      ),
                      itemCount: _messages.length,
                      itemBuilder: (context, index) {
                        final message = _messages[index];
                        final isMe = currentUserId != null &&
                            message.senderId == currentUserId;
                        return _MessageBubble(
                          message: message,
                          isMe: isMe,
                        );
                      },
                    ),
                  ),
          ),
          _MessageComposer(
            controller: _textController,
            isSending: _isSending,
            onSend: _sendMessage,
          ),
        ],
      ),
    );
  }
}

class _MessageBubble extends StatelessWidget {
  const _MessageBubble({required this.message, required this.isMe});

  final RestsendChatLog message;
  final bool isMe;

  @override
  Widget build(BuildContext context) {
    final text = message.content.text.isNotEmpty
        ? message.content.text
        : (message.content.placeholder.isNotEmpty
            ? message.content.placeholder
            : '[${message.content.contentType}]');
    final alignment = isMe ? Alignment.centerRight : Alignment.centerLeft;
    final bubbleColor = isMe
        ? Theme.of(context).colorScheme.primary
        : Theme.of(context).colorScheme.surfaceContainerHighest;
    final textColor = isMe
        ? Theme.of(context).colorScheme.onPrimary
        : Theme.of(context).colorScheme.onSurface;
    return Align(
      alignment: alignment,
      child: ConstrainedBox(
        constraints:
            BoxConstraints(maxWidth: MediaQuery.of(context).size.width * 0.7),
        child: Card(
          color: bubbleColor,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            child: Text(
              text,
              style: TextStyle(color: textColor),
            ),
          ),
        ),
      ),
    );
  }
}

class _MessageComposer extends StatelessWidget {
  const _MessageComposer({
    required this.controller,
    required this.isSending,
    required this.onSend,
  });

  final TextEditingController controller;
  final bool isSending;
  final VoidCallback onSend;

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      top: false,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 8, 12, 12),
        child: Row(
          children: [
            Expanded(
              child: TextField(
                controller: controller,
                textInputAction: TextInputAction.send,
                onSubmitted: (_) => onSend(),
                decoration: const InputDecoration(
                  hintText: 'Type a message…',
                  border: OutlineInputBorder(),
                  isDense: true,
                ),
              ),
            ),
            const SizedBox(width: 8),
            FilledButton.icon(
              onPressed: isSending ? null : onSend,
              icon: isSending
                  ? const SizedBox(
                      width: 14,
                      height: 14,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.send),
              label: const Text('Send'),
            ),
          ],
        ),
      ),
    );
  }
}
