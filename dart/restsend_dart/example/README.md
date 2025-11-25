# restsend_flutter_demo

Flutter demo application showing how to use the local `restsend_dart` bindings.
The app lets you:

- Provide endpoint/user/password credentials and initialize the Rust client.
- List existing conversations, refresh locally cached entries, and trigger an
  incremental sync via the SDK callbacks.
- Open a conversation, load cached chat logs, sync the latest messages, and send
  simple text messages.

## Prerequisites

- Rust toolchain installed (needed to build the FFI library).
- Flutter 3.24+ with Dart 3.5+.
- Android/iOS/desktop toolchains depending on target platform.

## Setup

1. Build the bindings from the repo root (only needed when the Rust API changes):

	```bash
	cargo check
	(or the usual flutter_rust_bridge codegen flow)
	```

2. Fetch dependencies for both the Dart package and the example:

	```bash
	cd dart/restsend_dart
	dart pub get
	cd example
	flutter pub get
	```

3. Run the demo:

	```bash
	flutter run
	```

Fill in the login form with valid endpoint, user ID, and password values from your
restsend environment. Optionally set a custom DB path/name for debugging. After
sign-in you can sync conversations and send messages from the chat view.
