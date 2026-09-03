# restsend 全栈开发记录（2025-05 ~ 2026-09）

> 统计范围：自 2025-05-01 起
>
> | 仓库 | 角色 | 提交数 | 变更量 |
> |---|---|---|---|
> | restsend-rs | 客户端 SDK（WASM/SQLite/IndexedDB + FFI 多端） | 109 | +72,268 / -40,001 |
> | restsend | 服务端（Go） | 26 | +2,756 / -378 |

## 总览：各阶段主线

| 时间 | 客户端（restsend-rs） | 服务端（restsend） |
|---|---|---|
| 2025-05~06 | ping/pong 心跳、并发重构（RwLock）、未读修复 | 渐进式稳定性修复 |
| 2025-09~10 | **WASM 性能专项**（IndexedDB/批量更新/同步协议）、ClientOption | 会话序号、Tags 修缮 |
| 2025-11~12 | 会话分类过滤、服务端 unreadCount、**dart SDK**、mark unread | **未读/分类/seen 体系**（年度功能重心） |
| 2026-01~03 | **iOS/macOS/Android 全端落地**、会话合并策略 | PascalCase API 重构、webhook relation 参数 |
| 2026-04~05 | 移除 flutter/dart crate、**tags/extra 模型 + 迁移**、demo 模式、架构文档 | （空窗） |
| 2026-06~09 | 合并逻辑持续打磨、**内存会话 + LRU 缓存**（1.2.45） | **LRU session、idle timeout、SIP relay** |

---

# 一、客户端 restsend-rs（109 次提交）

## 2025-05 ~ 06：心跳与并发重构

- `da0c5a6` / `1ca9b63` **05-26** feat: **ping/pong 心跳**、lastAliveAt
- `c6b9d99` **06-04** bump 1.1.4
- `15c4e5e` **06-05** 连接处理锁从 Mutex 重构为 **RwLock**；`c5d85ce` 会话更新非阻塞化（1.1.5/1.1.6）
- `835067d` **06-12** 新消息 is_countable 判定
- `4226b9d` / `bd1d034` **06-13** 未读数修复、移除 try_sync_chat_logs（1.1.9）
- `b4e3ae9` **06-16** unread issue 修复
- `2f5a8ce` **06-19** fix: 关闭时 websocket 安全退出

## 2025-09：WASM 性能专项（高频优化期）

- `26b3ec7` **09-04** wasm 增加 syncFirstPageConversations
- `7cec3e2` / `a8638c6` **09-05** sync_conversations 增加 `beforeUpdatedAt` 参数（1.2.7）
- `2d58b19` **09-11** on_conversation_updated 携带 total（可选）；`93a7838` **09-15** on_success 回调带总大小
- `aabc7d7` / `4ebe87d` **09-16** IndexedDB 查询 continue 问题修复（1.2.10）
- `8109f47` **09-17** 远端会话同步计数追踪
- `608ab37` / `c39c2bb` / `46026a9` / `cdf7db6` / `2ba491f` **09-19** 一天内密集优化：**batch_update 免预查询**、减少 store 创建、conversation 拉取优化、减少 chat log 事务、**废弃 JSFuture**、未读构建日志
- `ffc81b4` **09-19** externref shims 更新、IndexedDB 表关闭时清理
- `b6b6a8b` / `c06457a` **09-25~26** 拉取逻辑精简、同步 last readable message

## 2025-10：稳定性与配置化

- `42dc91e` **10-11** feat: **readonly_table**
- `1a8ea21` / `adce4fb` **10-13** chat log 同步修复（1.2.15）
- `01b443f` **10-17** fix: websocket 关闭时崩溃
- `af37d35` **10-21** 会话拉取错误处理增强（1.2.17）
- `054080d` / `b37a615` **10-23** **ClientOption** 配置模型、RequestGuard 安全释放 IndexedDB

## 2025-11：分类过滤 + dart SDK（功能期）

- `9fc6729` / `046571b` / `1f8feae` **11-05** WASM 绑定重构（认证/会话管理）、client store 整理（1.2.21）
- `aa13800` **11-06** feat: **会话分类过滤**（category filters）
- `d31cd9f` **11-13** 已删除会话管理改进
- `cd3d912` / `995b530` **11-17** **未读数改用服务端下发**，不再本地构建
- `6b2db0e` **11-18** 构建配置更新、会话管理 API 扩充
- `7dffc6d` **11-25** feat: **dart client sdk**；`4530e75` 运行时初始化修复

## 2025-12：mark unread 与 kind 参数

- `ee56651` / `7a7430b` / `10f0775` **12-08** 存储层错误处理增强、SqliteTable 日志、**Android/iOS 平台日志初始化**
- `00f87e4` / `5d8ba1c` **12-17~18** feat: **markConversationUnread**
- `e16f641` **12-29** topic 创建/更新增加 **`kind` 参数**（1.2.28，与服务端同步）

## 2026-01 ~ 03：全平台覆盖（iOS / macOS / Android）

- `7d9ce31` **01-16** restsend_dart 插件 **macOS/iOS 支持**（构建脚本、配置）
- `7000621` / `902c6a6` / `c77a872` **01-27** **macOS/iOS 构建体系**：macOS universal dylib、podspec vendored libraries、iOS 改用动态库
- `6a93e4d` **01-30** 会话合并策略：**本地消息较新时优先**（含测试）
- `cc00cf2` **02-04** RestsendClient 日志初始化与事件处理
- `8fadead` **02-05** macOS Podfile、Flutter root 检测
- `0ce7171` **03-10** feat: **Android 支持**（build script、Gradle、plugin）
- `f5d57e5` / `8147a5a` / `c51afc5` / `3b710a7` / `1a6d536` **03-12~14** Android/iOS 构建连环修复（kotlin typo、build_ios.sh、动态库处理）

## 2026-04：架构瘦身与缓存

- `22b1bf1` **04-20** feat: **on_ping_failed 回调**
- `c141c2b` / `bf6ed47` **04-22** **移除 flutter 与 restsend-dart crate**（dart 改走独立 SDK 路线）
- `cf8759a` **04-22** chat log 处理与**缓存机制**增强
- `13485ce` / `1632f46` / `31c06e9` **04-23** 后端 release、代码重构

## 2026-05：数据模型升级 + demo 模式 + 测试体系（最密集月份，23 次）

- `e3b299e` **05-06** 用户创建功能、admin 界面增强
- `b34fb4f` / `4580f4c` / `ecfca09` / `2ab9780` **05-07** **Dockerfile** 引入与迭代
- `8d1641b` / `f1864b7` / `67b72f2` / `9c7912e` / `8415b43` **05-07** **demo SPA 模式**：demo 端点/HTML、静态路径解析、demo fixtures、客户端 IP 提取
- `3445d73` **05-08** **Migrator 迁移框架**（InitSchema 建表）、IndexedDB 类型化重构
- `7581418` **05-08** **架构文档**：能力模型、分层运行时、WebSocket 流、消息同步、会话模型、集群路由、SDK 演进图
- `7a75049` / `f6fb67c` **05-11** AtomicBool 防 sync_conversations 重入、合并时**保留本地未读数**
- `87ec301` / `e4467c9` / `afaebd9` **05-12** **LocalTestServer** 测试基建、会话与 WebSocket 投递综合测试、**update_contact 端点**
- `90bd3e2` / `dfb8007` / `22a43bc` / `859d4a2` **05-21** **会话模型增加 tags/extra 字段** + DB 迁移、**createWorkerClient（Web Worker 支持）**、错误日志增强（1.2.34~36）

## 2026-06 ~ 08：合并逻辑打磨与内存治理

- `9dfb4e0` **06-05** 合并逻辑改用本地状态（1.2.1）
- `a83c9cb` / `0a2330e` **06-25 / 07-01** 会话合并逻辑与类型更新（1.2.38/39）
- `79accc5` **07-14** 所有 on_conversations_updated 路径应用 **stale last_message 修复**（healing）
- `86b8e29` **07-29** fix: 等待 put 结果
- `3d3ac47` / `1db1b31` **07-31** extra 覆盖问题修复与回退
- `043644f` **08-03** **extra 按 key 合并** + 防 stale 服务端快照
- `61a1fd9` **08-03** **v1.2.45：会话默认驻留内存、时间维度消息清理、LRU 有界缓存**、ws version 查询

---

# 二、服务端 restsend（26 次提交）

## 2025-05 ~ 08：渐进式修复

- `d2ab293` **05-26** dup chat id 修复
- `a2f7cbf` **06-05** 关闭时不将 client.ctx 置 nil
- `f22295b` **07-07** 处理会话前检查 topic 成员资格
- `0ce4758` **08-09** Conversation 复合索引定义更新

## 2025-10：会话序号/标签修缮

- `71a02d7` **10-14** `CreateOrUpdateConversationLastSeqWithOwnerID` 支持可选 timestamp 参数
- `9e7bbe6` **10-17** `UpdateConversationForm` 的 Tags 字段改为必填

## 2025-11：会话未读 + 分类体系（年度功能重心）

- `4d52074` **11-06** 会话**分类过滤与聚合**：新增 `conversation_category_provider.go`（109 行），+238 行
- `2840976` **11-06** 修正拼写 `Categories`
- `60c754c` **11-17** **会话未读追踪框架**（+452 行）：unread_provider、category_hooks、list_hooks、模型扩展
- `3efc650` **11-17** ConversationRequest 增加 **seen 追踪**，PushUsers 更新
- `a0955f0` / `6fcbca5` **11-21 / 11-24** unreadable 条件反转、unreadable 内容不计入未读
- `a2ca604` **12-01** 消息处理发出 **read 信号**

## 2025-12：mark unread 与 topic kind

- `f309ab3` **12-17** **「标记会话为未读」端点**；`faac90b` **12-18** 补全 OpenAPI + 测试（+108 行）
- `78ecd32` **12-29** topic 模型/表单增加 **`kind` 字段**（与客户端 1.2.28 呼应）
- `68fb9be` / `36c52be` carrot 依赖跟进

## 2026-03：API 规范化

- `f2fd8bf` **03-09** 方法名统一 **PascalCase**（14 个文件，破坏性重构）
- `701df4a` / `d7c1d0e` **03-09 / 03-10** `OnAccessUserProfile` 增加 **relation 参数**

## 2026-06 ~ 09：连接层健壮性 + SIP 场景

- `2699c38` **06-26** **LRU 会话存储**：`session_store.go` + 708 行测试（+922 行，年度最大提交），TTL + 淘汰
- `78c9ed8` **07-30** **WebSocket 空闲超时**（+114/-60）
- `05fa498` **09-01** **SIP relay**：`handler_sip_relay.go`（163 行）+ 240 行测试，`SIP_RELAY_PBX_WS` 在 `/api/connect` 与 PBX WS 间透传原始 SIP（+413 行）
- `35594ec` **09-01** client tx 创建竞态修复、响应静默丢失修复
- `fdf2ff4` **09-01** CI：CNB 镜像流水线，Dockerfile.cn 合并

---

# 三、两端协同观察

1. **强联动的功能对**（客户端与服务端同期实现）：
   - ping/pong：服务端 2025-04 ping chat request → 客户端 2025-05 ping/pong 心跳
   - unread 体系：服务端 2025-11 unread provider/分类 → 客户端 2025-11 改用服务端 unreadCount + 分类过滤
   - mark unread：两端同步 2025-12-17/18
   - topic kind：两端同步 2025-12-29
   - LRU 治理：服务端 2026-06 LRU session → 客户端 2026-08 LRU 缓存（1.2.45）

2. **开发节奏**：客户端全程高频（2026-05 单月 23 次）；服务端呈「脉冲式」——2025-11~12 未读体系、2026-06~09 连接层，其余时段低频维护。

3. **技术路线演变**：客户端从 WASM 单端 → dart/Flutter 尝试 → **Rust core + FFI 全端覆盖**（iOS/macOS/Android/Web Worker），2026-04 果断移除 flutter/dart crate 收敛路线；服务端从功能堆叠转向 **hook/provider 可扩展架构** 与连接层健壮性。

4. **年度关键词**：未读一致性（unreadable/seen/read 信号）、会话分类（category/kind）、多端覆盖、内存与缓存治理（LRU）、SIP 新场景。
