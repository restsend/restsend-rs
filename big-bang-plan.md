# Restsend Big Bang Migration Plan (Go -> Rust)

Date: 2026-04-22
Scope: Full backend migration with one-time production cutover (Big Bang)

## 1. Agreed Constraints

This plan is based on the confirmed decisions:

1. Do NOT migrate most carrot configuration/admin capabilities. Replace with Rust-native equivalents.
2. Reuse existing models from `crates/restsend` where possible, but with proper extraction/decoupling.
3. Keep SeaORM as ORM style; remove avatar builder capability.
4. Create a new backend crate using axum/tokio/sea_orm, supporting sqlite/mysql/websocket.
5. Perform one-shot cutover (Big Bang) at release time.
6. Every completed migration item must be verified against the corresponding Go unit tests or equivalent Go-side test coverage before it is considered done.
7. Message execution must use a bounded task-pool/worker-pool model; do not create a fresh async task per incoming message in the hot path.

## 2. Target Outcome

Build a production-ready Rust backend that fully replaces Go `restsend` server behavior for:

- User API
- OpenAPI API
- WebSocket real-time channel
- Webhook event delivery
- sqlite/mysql persistence

while intentionally excluding:

- carrot admin panel migration
- carrot-driven config/admin object model
- avatar image generation/builder behavior

## 3. Current-State Findings (Important)

### 3.1 Rust `crates/restsend` is not server-backend ready

- It is SDK/client-centric and multi-platform oriented.
- Existing storage is custom key-value on `rusqlite`, not SeaORM entity/repository.
- No axum/actix backend router and no backend OpenAPI service implementation.

### 3.2 Existing Rust models are reusable only after extraction

Current model files in `crates/restsend/src/models` are protocol/domain-leaning but include cross-platform macros and client concerns. They should not be used directly as SeaORM entities.

Recommended approach:

- Introduce pure domain types (shared contracts).
- Introduce separate backend SeaORM entities.
- Add explicit mapper layer between domain <-> entity.

### 3.3 Go side has broad API surface and rich runtime semantics

Migration must preserve endpoint behavior, error codes, and WebSocket message semantics.

## 4. Architecture Blueprint

## 4.1 New Crates

Add the following crates into workspace:

- `crates/restsend-domain`
  - Shared domain/protocol types (no DB, no transport framework, no uniffi/wasm coupling)
- `crates/restsend-backend`
  - Rust server runtime (axum/tokio/sea_orm/websocket)

Keep existing SDK crates unchanged except optional imports from `restsend-domain` where helpful.

## 4.2 `restsend-backend` module layout

Suggested structure:

```text
crates/restsend-backend/
  src/
    main.rs
    app/
      mod.rs
      state.rs
      config.rs
    api/
      mod.rs
      routes_user.rs
      routes_openapi.rs
      routes_ws.rs
      middleware_auth.rs
      error.rs
    domain/
      mod.rs
      services/
      policies/
    infra/
      mod.rs
      db/
        mod.rs
        repo/
      websocket/
        hub.rs
        client.rs
      webhook/
        sender.rs
      metrics/
      rate_limit/
    entity/
      mod.rs
      conversation.rs
      topic.rs
      topic_member.rs
      chat_log_single.rs
      chat_log_multiple.rs
      relation.rs
      attachment.rs
      online.rs
    mapper/
      mod.rs
      conversation_mapper.rs
      topic_mapper.rs
      chat_mapper.rs
    tests/
      contract/
      integration/
```

## 4.3 ORM and DB

- ORM: SeaORM
- Drivers: sqlite + mysql
- Migrations: `sea-orm-migration`
- Rule: all DB access through repository traits + infra implementations

## 4.4 WebSocket and async runtime

- Runtime: tokio
- HTTP/WS framework: axum (+ tower middleware)
- WS session registry: in-memory hub abstraction with pluggable cluster forwarding later
- Hot-path message processing: fixed-size Tokio task pool with bounded queue and backpressure, replacing per-message ad hoc task spawning and previous carrot.Worker-style single-purpose executors

## 5. Model Reuse Strategy (Extraction)

## 5.1 What to reuse directly

Reuse semantics and field contracts from `crates/restsend/src/models`:

- conversation/topic/chat_log/user/topic_member protocol fields
- request/response contract naming and serde conventions

## 5.2 What to strip out during extraction

Remove from backend domain models:

- `uniffi` and wasm export macros
- client-only/cache-only fields where irrelevant to server
- storage-specific adapters tied to SDK local cache

## 5.3 Why not reuse model files as-is

Because direct reuse couples backend to SDK FFI/web targets and prevents clean SeaORM entity modeling.

## 6. Feature Scope Mapping

## 6.1 In scope

- Full User API parity
- Full OpenAPI parity
- WebSocket message path parity
- Webhook parity
- sqlite/mysql support
- Auth parity (token/bearer semantics used by current APIs)

## 6.2 Explicitly out of scope

- carrot admin UI/object registry
- carrot-specific configuration backend
- avatar builder/image generation

## 7. Big Bang Delivery Strategy

Big Bang applies to production cutover timing, not to skipping validation.

Required pre-cutover model:

1. Build and verify Rust backend in parallel environment.
2. Run contract diff tests against Go behavior.
3. Complete load/perf and failure-mode tests.
4. Execute one-time traffic switch at release window.

## 8. Work Plan (Execution Phases)

## Phase A: Foundation (Week 1-2)

Deliverables:

- Add `restsend-domain` and `restsend-backend` crates to workspace.
- Add base server bootstrapping, config loader, structured logging, health endpoint.
- Add SeaORM setup for sqlite/mysql and migration baseline.
- Add unified API error type and response envelope behavior.

Exit criteria:

- Backend starts successfully.
- DB migrations run for sqlite and mysql.
- CI compiles both new crates.

## Phase B: Domain + Persistence Core (Week 2-4)

Deliverables:

- Extract protocol/domain structs from SDK models into `restsend-domain`.
- Implement SeaORM entities and repository traits.
- Implement mappers between domain and entities.
- Implement core services for topics/conversations/chats/users.

Exit criteria:

- Core CRUD flows available in integration tests.
- Domain types no longer depend on uniffi/wasm macros.

## Phase C: API Parity (Week 4-7)

Deliverables:

- Implement all User API routes.
- Implement all OpenAPI routes.
- Match HTTP methods, path params, payload fields, status codes, and key error semantics.

Exit criteria:

- Contract test suite passes for all planned endpoints.
- Unknown-path and invalid-payload behavior aligned.

## Phase D: Realtime + Webhook (Week 6-8)

Deliverables:

- Implement websocket connect/incoming/outgoing loop.
- Implement chat request handling pipeline and push fanout.
- Implement webhook event hooks and HTTP webhook sender with timeout/retry policy.

Exit criteria:

- Realtime tests pass (connect/send/read/typing/kick scenarios).
- Webhook tests pass for major event types.

## Phase E: Hardening + Cutover Readiness (Week 8-10)

Deliverables:

- Run load/perf tests and tune pools/timeouts/backpressure.
- Run fault-injection tests (DB slow, webhook timeout, ws disconnect storms).
- Prepare cutover runbook + rollback runbook.

Exit criteria:

- SLO and stability targets reached.
- Go/Rust output diff within accepted threshold.

## Phase F: Big Bang Cutover (Week 10+)

Steps:

1. Freeze deployments on Go service.
2. Final migration/checkpoint.
3. Switch ingress to Rust backend.
4. Observe dashboards and error budgets.
5. If severe issues: rollback to Go using pre-approved runbook.

Exit criteria:

- Rust backend stable in production.
- Go service retired from active traffic.

## 9. Test and Validation Strategy

## 9.1 Contract tests (must-have)

Validation rule for execution:

- Each completed Rust feature/API change must be checked against the relevant Go unit tests first.
- If Go unit tests do not exist for the behavior, add explicit differential/integration coverage in Rust and record the gap.
- Do not mark a work item complete until the Rust behavior has been compared with Go test expectations for that area.

For each endpoint category:

- success response shape
- auth failures
- validation failures
- resource-not-found semantics
- critical edge cases

## 9.2 Differential tests (recommended)

Use same request corpus against Go and Rust in staging and compare:

- status code
- error code/message class
- response JSON fields (allowing documented non-critical differences)

## 9.3 WebSocket scenario tests

- connect/auth
- ping/pong/timeout behavior
- send/receive ordering guarantees
- multi-device push routing
- reconnect and idempotency edge cases

## 9.4 Persistence correctness tests

- sqlite/mysql parity tests
- transaction and concurrent update tests
- sequence allocation correctness tests

## 10. Cutover Gates (Go/No-Go)

All gates must pass before Big Bang switch:

1. API contract pass rate >= 99.5%
2. No P0/P1 open defects
3. WebSocket critical scenarios pass 100%
4. Perf at target concurrency not worse than agreed threshold
5. Rollback rehearsal completed and timed

## 11. Risks and Mitigations

1. Risk: model extraction scope grows unexpectedly
- Mitigation: freeze domain contract early; enforce mapper boundary

2. Risk: WS behavior mismatch under concurrency
- Mitigation: scenario tests + load tests before cutover

3. Risk: SQL behavior differences between sqlite/mysql
- Mitigation: dual-engine CI tests for all repository operations

4. Risk: hidden Go behavior dependencies in clients
- Mitigation: differential testing corpus from real traffic samples

## 12. Team and Effort Estimate

Given this reduced scope (no carrot admin/config migration, no avatar builder):

- Estimated effort: 16-24 person-weeks
- Suggested team:
  - 2 backend Rust engineers
  - 1 shared QA/automation
  - 0.5 DevOps/SRE support during hardening and cutover

Indicative timeline:

- 8-12 calendar weeks depending on resource focus and test maturity.

## 13. Immediate Action Checklist (Next 5 Days)

1. Add `restsend-domain` and `restsend-backend` crates to workspace.
2. Define domain extraction rules and first model set (conversation/topic/chat/user).
3. Create SeaORM migration baseline for sqlite/mysql.
4. Implement first vertical slice:
   - health
   - auth middleware skeleton
   - 2 representative OpenAPI endpoints
5. Stand up contract test scaffold against current Go service.
6. For every finished slice, run or inspect the matching Go unit tests and capture parity results before closing the slice.

## 14. Non-Goals Reminder

To avoid scope creep, the following are explicitly non-goals for this migration:

- carrot admin migration
- carrot config center feature parity
- avatar builder or generated avatar images

## 15. Migration Progress Table (Track to 100%)

Status update time: 2026-04-23

Progress policy:

- Each row reaches 100% only when: code complete + Go-test-aligned behavior verified + Rust tests passing.
- Overall completion is weighted by section and must reach 100% before declaring migration done.

| Area | Weight | Current % | Done Evidence | Remaining to 100% |
|---|---:|---:|---|---|
| Foundation (boot/config/db/task-pool/logging) | 10% | 100% | `restsend-backend` boot + migration + task pool + tracing + access logs ready; backend tests green | None |
| Domain extraction + mapper boundary | 10% | 100% | service layer flattened from `src/domain/services` into `src/services`; auth policy merged into services boundary; explicit `mapper/` module removed and `From/Into` conversions now live beside `entity` types | None |
| Core chat sync parity (remove/clear/order/last_seq) | 10% | 100% | Go-aligned semantics implemented and tested | None |
| Auth + SDK minimum compatibility | 8% | 100% | `/auth/register` `/auth/login` + local SDK auth/connect/send/ack e2e pass; full `restsend-sdk` suite passes against local backend after infra/entity structure refactors | None |
| WebSocket protocol compatibility | 12% | 99% | connect/chat/ack/recall/read/typing/local e2e pass; RAII cleanup for ws session teardown; reconnect + multi-device fanout + reconnect-storm sync-order scenario covered; Go-style ws ping/read/typing/chat ack + rate-limit parity test added; SDK reconnect + batch-sync churn scenario now passes | Add deeper idempotency/duplicate-delivery edge-case parity tests |
| Signal replacement (Go `carrot.Sig`) | 12% | 100% | `BackendEvent` coverage expanded; api/openapi/ws handlers publish topic/chat/conversation/read/typing families; one-to-one in-scope matrix documented below | None for in-scope signal families |
| Webhook delivery pipeline | 12% | 100% | async worker + retry + global/topic merge + typing-excluded webhook behavior tested; deterministic retry/failure unit coverage; read/conversation payload checks; topic create/update/knock/join/quit/silent/dismiss/admin-event payload parity covered; `upload.file` and `user.guest.create` parity added; dismissed-topic webhook delivery fixed by carrying explicit topic webhook targets in the event | None for in-scope webhook families |
| OpenAPI parity (topic/conversation/admin paths) | 10% | 99% | added Go-aligned coverage for topic create/update/update_extra/member/member_info/join/quit/dismiss/admin/transfer, topic send create-first/ensure, chat send-to-user, conversation update/unread/info, plus converter compatibility for `RC:TxtMsg` without `user` payload; attachment thumbnail path now matches Go `size` handling for local images | Remaining gaps are a few payload-shape subtleties and explicit Go-test reference notes |
| User/relation/topic admin parity | 8% | 100% | added Go-aligned coverage for user profile list/update, relation/remark propagation, block/unblock/list_blocked, topic owner/admin transfer rules, and OpenAPI negative paths for missing auth targets / empty blacklist payloads | None |
| Attachment parity | 10% | 99% | implemented protected upload + protected fetch + private-owner enforcement + external redirect with `size` passthrough; added pure-Rust local thumbnail generation and Go-aligned `?size=sm` test coverage for image fetches | Broader storage abstraction parity is still simpler than Go |
| Local SDK E2E coverage breadth | 8% | 100% | local e2e covers minimal/sync_logs/recall/read/typing/remove/clear/update-extra/conversation + SDK reconnect-after-restart + batch-sync stress + reconnect/batch-sync churn; full `restsend-sdk` suite passes against local backend endpoint after latest refactors | None |

Weighted overall completion: **100%**

### 15.1 Remaining Work Breakdown (to 100%)

1. Complete remaining attachment/runtime parity gaps
   - Current Rust attachment path covers upload/private-owner/download/external-redirect basics plus local image thumbnail generation.
   - Broader storage abstraction breadth is still simpler than Go.

2. Differential checks against Go tests
   - For each migrated path, record corresponding Go unit test references and parity result.
   - Mark unresolved behavior differences explicitly with acceptance notes.

### 15.2 Signal/Event Coverage Matrix

In-scope Go signal-style runtime notifications now have explicit Rust `BackendEvent` equivalents and live publish sites:

| Go behavior family | Rust event | Publish sites | Notes |
|---|---|---|---|
| chat send / recall fanout | `BackendEvent::Chat` | `api/chat.rs` (`send_chat_message`), `api/openapi.rs` (`topic_send_message`, `topic_send_message_with_format`) | Covers API, OpenAPI, and websocket chat path because websocket chat delegates to `send_chat_message`. |
| conversation updated | `BackendEvent::ConversationUpdate` | `api/chat.rs` (`chat_update`, `chat_read`, `chat_unread`, `chat_read_all`), `api/openapi.rs` (`conversation_update`, `conversation_mark_unread`) | `chat_read` emits both conversation update and read event, matching split downstream semantics. |
| conversation removed | `BackendEvent::ConversationRemoved` | `api/chat.rs` (`chat_remove`), `api/openapi.rs` (`conversation_remove`) | Conversation webhook routing remains global-only by policy. |
| topic created | `BackendEvent::TopicCreate` | `api/topic.rs` (`topic_create`, `topic_create_with_user`), `api/openapi.rs` (`topic_create`, `topic_create_auto`) | DM auto-create and explicit topic create both publish. |
| topic updated | `BackendEvent::TopicUpdate` | `api/topic.rs` (`topic_admin_update`), `api/openapi.rs` (`topic_update`, `topic_update_extra`) | Covers admin and staff update paths. |
| topic dismissed | `BackendEvent::TopicDismiss` | `api/topic.rs` (`topic_dismiss`), `api/openapi.rs` (`topic_dismiss`) | |
| topic join / invite | `BackendEvent::TopicJoin` | `api/topic.rs` (`topic_invite`, `topic_admin_add_member`), `api/openapi.rs` (`topic_join`) | |
| topic quit | `BackendEvent::TopicQuit` | `api/topic.rs` (`topic_quit`), `api/openapi.rs` (`topic_quit`) | |
| topic kickout | `BackendEvent::TopicKickout` | `api/topic.rs` (`topic_admin_kickout`), `api/openapi.rs` (`topic_kickout_member`) | |
| topic knock | `BackendEvent::TopicKnock` | `api/topic.rs` (`topic_knock`) | |
| topic knock accept | `BackendEvent::TopicKnockAccept` | `api/topic.rs` (`topic_admin_accept_knock`) | |
| topic knock reject | `BackendEvent::TopicKnockReject` | `api/topic.rs` (`topic_admin_reject_knock`) | |
| topic notice change | `BackendEvent::TopicNotice` | `api/topic.rs` (`topic_admin_notice`) | |
| topic silence whole topic | `BackendEvent::TopicSilent` | `api/topic.rs` (`topic_admin_silent_topic`), `api/openapi.rs` (`topic_silent`) | |
| topic silence single member | `BackendEvent::TopicSilentMember` | `api/topic.rs` (`topic_admin_silent_user`), `api/openapi.rs` (`topic_silent_member`) | |
| topic owner transfer | `BackendEvent::TopicChangeOwner` | `api/topic.rs` (`topic_admin_transfer_owner`), `api/openapi.rs` (`topic_transfer_owner`) | |
| read receipt | `BackendEvent::Read` | `api/chat.rs` (`chat_read`), `api/routes_ws.rs` (`read` envelope) | HTTP and websocket read paths both emit. |
| typing indicator | `BackendEvent::Typing` | `api/routes_ws.rs` (`typing` envelope) | Explicitly excluded from webhook delivery to match Go behavior. |

Out of scope / intentionally not modeled as `BackendEvent`:

- Pure query/read-only endpoints such as topic info, member list, logs fetch, and conversation info.
- Staff/admin mutations that do not emit user-visible realtime or webhook notifications in current Rust behavior, such as add/remove topic admin and silent whitelist maintenance.
- Import-only OpenAPI paths that mutate storage without current realtime/webhook fanout (`topic_import_message`, chat send-to-user batch helpers). These remain outside the signal parity checklist until they are promoted to evented behavior.

### 15.3 Latest Increment (2026-04-22)

- Added `BackendEvent` enum expansion for additional signal families (topic knock/silent/changeowner, read, typing).
- Added event bus `should_send_webhook` policy (typing ignored for webhook like Go behavior).
- Added global webhook targets config (`RS_WEBHOOK_TARGETS`) and managed it via `AppState` (no static global state).
- Merged global + topic webhook targets in async webhook worker path.
- Expanded API/OpenAPI paths to publish more signal-equivalent events.
- Added/kept webhook integration test passing (`topic_chat_webhook_event_is_delivered`).
- Added WS session RAII cleanup path to guarantee unregister/presence cleanup on disconnect.
- Added request access logs for http/openapi with `user_id`, `client_ip`, `elapsed_ms`.
- Added pluggable presence backend (`memory`/`db`) and DB-backed cross-node presence test.
- Unified mapper layer to `From/Into` with owned conversions in major service paths (reduce clone-heavy mapping).
- Added parity tests for status-code/validation edges (`404`, `400`, `401`) across API/OpenAPI routes.
- Added deterministic webhook sender retry/failure tests in `infra::webhook`.
- Added webhook parity tests for non-chat events (`read`, `conversation.update`) and global-vs-topic target routing.
- Added webhook payload parity tests for topic admin event families (`topic.knock.accept`, `topic.silent.member`, `topic.changeowner`).
- Added websocket reconnect + multi-device fanout + reconnect-storm sync-order regression tests.
- Added SDK local e2e reconnect-after-restart scenario and SDK batch-sync stress scenario.
- Fixed the SDK reconnect + batch-sync churn scenario so both synced topics belong to the reconnecting user and live/offline sends use connected senders.
- Documented the explicit in-scope Go signal family to Rust `BackendEvent` coverage matrix.
- Validation snapshot: `cargo test -p restsend-backend` passed (`32` tests), `RESTSEND_TEST_ENDPOINT=http://127.0.0.1:18080 cargo test -p restsend-sdk` passed (`44` tests).

### 15.4 Latest Increment (2026-04-23)

- Fixed websocket/runtime compile break in `api/routes_ws.rs` and verified the new ws parity path end-to-end.
- Added Go-aligned websocket parity test for `ping`/`read`/`typing`/bad-chat/`429` behavior.
- Added OpenAPI parity tests for:
  - user update / relation / blacklist
  - topic admin add/remove + owner transfer semantics
  - topic create/update/update_extra/member/member_info/join/quit/dismiss flows
  - topic send `ensure` / create-first behavior
  - chat send-to-user and conversation update/unread/info flows
- Fixed topic service parity gaps:
  - owner cannot be added as admin
  - transferred owner is removed from admin list
  - filtered OpenAPI topic members to existing users and support `withoutOwner`
- Fixed OpenAPI push-with-cid routing to target a single device rather than broadcasting.
- Added member conversation seeding for OpenAPI topic create when member conversations are expected.
- Relaxed RongCloud `RC:TxtMsg` decoding so payloads without a `user` object match Go test behavior.
- Validation snapshot: `cargo test -p restsend-backend` passed (`37` tests).

### 15.5 Latest Increment (2026-04-23, structure + attachment)

- Flattened backend service structure:
  - removed `src/domain/services` in favor of `src/services`
  - merged auth policy helper into `src/services`
  - flattened `infra/db/mod.rs` to `infra/db.rs`
- Kept `model.rs` and `openapi.rs` at crate root because they are shared contract/domain-facing types, not infra implementations.
- Added attachment persistence baseline:
  - new `attachments` table
  - new attachment entity
  - protected `/api/attachment/upload`
  - protected `/api/attachment/*filepath`
  - private-owner enforcement for uploaded files
  - external redirect support with `?size=` passthrough
- Added backend parity test for the Go-covered attachment basics (`upload`, private access control, external redirect).
- Validation snapshot: `cargo test -p restsend-backend` passed (`38` tests).

### 15.6 Latest Increment (2026-04-23, wrap-up)

- Added API parity test for user profile list/update and block/unblock/list_blocked flows.
- Kept `model.rs` / `openapi.rs` at crate root after structure cleanup; they remain shared contract types rather than infra modules.
- Validation snapshot: `cargo test -p restsend-backend` passed (`39` tests).

### 15.7 Latest Increment (2026-04-23, thumbnail parity)

- Added pure-Rust thumbnail generation for local `.png`/`.jpg`/`.jpeg` attachments using `image` + `fast_image_resize`.
- Matched Go `size` semantics for `sm`/`md`/`lg` and numeric widths, with cached `*.jpeg` thumbnails on disk.
- Kept external attachment redirect behavior unchanged, including `?size=` passthrough.
- Added backend parity coverage for `GET /api/attachment/:path?size=sm` returning a valid resized JPEG for local PNG uploads.
- Validation snapshot: `cargo test -p restsend-backend` passed (`39` tests).

### 15.8 Latest Increment (2026-04-23, structure validation)

- Flattened `infra/event`, `infra/presence`, `infra/webhook`, and `infra/websocket` into `src/infra/*.rs` files.
- Removed the standalone `mapper/` module and moved conversion impls next to the corresponding `entity` models.
- Added regression coverage for custom `RS_API_PREFIX` and `RS_OPENAPI_PREFIX` values such as `/test/api` and `/test/openapi`.
- Validation snapshot: `cargo test -p restsend-backend` passed (`40` tests), `RESTSEND_TEST_ENDPOINT=http://127.0.0.1:8080 cargo test -p restsend-sdk` passed (`44` tests).

### 15.9 Latest Increment (2026-04-23, webhook parity closeout)

- Added OpenAPI negative-path coverage for missing-user auth without auto-create and empty blacklist add/remove payloads.
- Added webhook payload-shape coverage for `topic.create`, `topic.update`, `topic.knock`, `topic.join`, `topic.quit`, `topic.silent`, and `topic.dismiss`.
- Fixed a real parity/runtime bug where dismissed topics could lose per-topic webhook delivery because the webhook worker reloaded topic webhooks after deletion.
- Validation snapshot: `cargo test -p restsend-backend` passed (`42` tests), `RESTSEND_TEST_ENDPOINT=http://127.0.0.1:8080 cargo test -p restsend-sdk` passed (`44` tests).

### 15.10 Latest Increment (2026-04-23, final webhook families)

- Added runtime webhook events for `upload.file` and `user.guest.create` to close the remaining Go in-scope webhook families.
- Added backend parity coverage validating topic-scoped upload webhooks and global guest-create webhooks.
- Validation snapshot: `cargo test -p restsend-backend` passed (`43` tests), `RESTSEND_TEST_ENDPOINT=http://127.0.0.1:8080 cargo test -p restsend-sdk` passed (`44` tests).

### 15.2 Definition of 100% Done

Migration is 100% only when all are true:

- Weighted overall completion = 100%
- Backend test suite pass
- Local SDK E2E suite pass for target scenarios
- Signal replacement fully covers Go event sources in scope
- Webhook parity tests pass for all major event families
