use sea_orm::Database;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};

pub async fn connect_db(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    Database::connect(database_url).await
}

pub async fn run_migrations(db: &DatabaseConnection) -> Result<(), DbErr> {
    let backend = db.get_database_backend();

    let create_users = Statement::from_string(
        backend,
        "
CREATE TABLE IF NOT EXISTS users (
  user_id VARCHAR(191) PRIMARY KEY,
  password TEXT NOT NULL DEFAULT '',
  display_name TEXT NOT NULL DEFAULT '',
  avatar TEXT NOT NULL DEFAULT '',
  source TEXT NOT NULL DEFAULT '',
  locale TEXT NOT NULL DEFAULT '',
  city TEXT NOT NULL DEFAULT '',
  country TEXT NOT NULL DEFAULT '',
  gender TEXT NOT NULL DEFAULT '',
  public_key TEXT NOT NULL DEFAULT '',
  is_staff BOOLEAN NOT NULL DEFAULT FALSE,
  enabled BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
"
        .to_string(),
    );

    let create_topics = Statement::from_string(
        backend,
        "
CREATE TABLE IF NOT EXISTS topics (
  id VARCHAR(191) PRIMARY KEY,
  name TEXT NOT NULL DEFAULT '',
  icon TEXT NOT NULL DEFAULT '',
  kind TEXT NOT NULL DEFAULT '',
  owner_id TEXT NOT NULL DEFAULT '',
  attendee_id TEXT NOT NULL DEFAULT '',
  members INTEGER NOT NULL DEFAULT 0,
  last_seq BIGINT NOT NULL DEFAULT 0,
  multiple BOOLEAN NOT NULL DEFAULT FALSE,
  source TEXT NOT NULL DEFAULT '',
  private BOOLEAN NOT NULL DEFAULT FALSE,
  knock_need_verify BOOLEAN NOT NULL DEFAULT FALSE,
  admins_json TEXT NOT NULL DEFAULT '[]',
  webhooks_json TEXT NOT NULL DEFAULT '[]',
  notice_json TEXT NOT NULL DEFAULT '{}',
  extra_json TEXT NOT NULL DEFAULT '{}',
  silent_white_list_json TEXT NOT NULL DEFAULT '[]',
  silent BOOLEAN NOT NULL DEFAULT FALSE,
  enabled BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
"
        .to_string(),
    );

    let create_conversations = Statement::from_string(
        backend,
        "
CREATE TABLE IF NOT EXISTS conversations (
  owner_id VARCHAR(191) NOT NULL,
  topic_id VARCHAR(191) NOT NULL,
  updated_at TEXT NOT NULL,
  sticky BOOLEAN NOT NULL DEFAULT FALSE,
  mute BOOLEAN NOT NULL DEFAULT FALSE,
  remark TEXT,
  unread BIGINT NOT NULL DEFAULT 0,
  start_seq BIGINT NOT NULL DEFAULT 0,
  last_seq BIGINT NOT NULL DEFAULT 0,
  last_read_seq BIGINT NOT NULL DEFAULT 0,
  last_read_at TEXT,
  multiple BOOLEAN NOT NULL DEFAULT FALSE,
  attendee TEXT NOT NULL DEFAULT '',
  members BIGINT NOT NULL DEFAULT 0,
  name TEXT NOT NULL DEFAULT '',
  icon TEXT NOT NULL DEFAULT '',
  kind TEXT NOT NULL DEFAULT '',
  source TEXT NOT NULL DEFAULT '',
  last_sender_id TEXT NOT NULL DEFAULT '',
  last_message_json TEXT NOT NULL DEFAULT '{}',
  last_message_at TEXT NOT NULL DEFAULT '',
  last_message_seq BIGINT NOT NULL DEFAULT 0,
  PRIMARY KEY(owner_id, topic_id)
);
"
        .to_string(),
    );

    let create_topic_members = Statement::from_string(
        backend,
        "
CREATE TABLE IF NOT EXISTS topic_members (
  topic_id VARCHAR(191) NOT NULL,
  user_id VARCHAR(191) NOT NULL,
  name TEXT NOT NULL DEFAULT '',
  source TEXT NOT NULL DEFAULT '',
  role TEXT NOT NULL DEFAULT 'member',
  silence_at TEXT,
  joined_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  extra_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY(topic_id, user_id)
);
"
        .to_string(),
    );

    let create_chat_logs = Statement::from_string(
        backend,
        "
CREATE TABLE IF NOT EXISTS chat_logs (
  id VARCHAR(191) PRIMARY KEY,
  topic_id VARCHAR(191) NOT NULL,
  seq BIGINT NOT NULL,
  sender_id VARCHAR(191) NOT NULL,
  content_json TEXT NOT NULL,
  deleted_by_json TEXT NOT NULL DEFAULT '[]',
  read BOOLEAN NOT NULL DEFAULT FALSE,
  recall BOOLEAN NOT NULL DEFAULT FALSE,
  source TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL
);
"
        .to_string(),
    );

    let create_relations = Statement::from_string(
        backend,
        "
CREATE TABLE IF NOT EXISTS relations (
  owner_id VARCHAR(191) NOT NULL,
  target_id VARCHAR(191) NOT NULL,
  is_contact BOOLEAN NOT NULL DEFAULT FALSE,
  is_star BOOLEAN NOT NULL DEFAULT FALSE,
  is_blocked BOOLEAN NOT NULL DEFAULT FALSE,
  remark TEXT NOT NULL DEFAULT '',
  source TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL,
  PRIMARY KEY(owner_id, target_id)
);
"
        .to_string(),
    );

    let create_topic_knocks = Statement::from_string(
        backend,
        "
CREATE TABLE IF NOT EXISTS topic_knocks (
  topic_id VARCHAR(191) NOT NULL,
  user_id VARCHAR(191) NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  message TEXT NOT NULL DEFAULT '',
  source TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'pending',
  admin_id TEXT NOT NULL DEFAULT '',
  PRIMARY KEY(topic_id, user_id)
);
"
        .to_string(),
    );

    let create_auth_tokens = Statement::from_string(
        backend,
        "
CREATE TABLE IF NOT EXISTS auth_tokens (
  token VARCHAR(191) PRIMARY KEY,
  user_id VARCHAR(191) NOT NULL,
  created_at TEXT NOT NULL,
  last_seen_at TEXT NOT NULL
);
"
        .to_string(),
    );

    let create_presence_sessions = Statement::from_string(
        backend,
        "
CREATE TABLE IF NOT EXISTS presence_sessions (
  user_id VARCHAR(191) NOT NULL,
  device VARCHAR(191) NOT NULL,
  node_id VARCHAR(191) NOT NULL,
  endpoint VARCHAR(191) NOT NULL DEFAULT '',
  updated_at_unix BIGINT NOT NULL,
  PRIMARY KEY(user_id, device)
);
"
        .to_string(),
    );

    let create_attachments = Statement::from_string(
        backend,
        "
CREATE TABLE IF NOT EXISTS attachments (
  path VARCHAR(191) PRIMARY KEY,
  file_name TEXT NOT NULL DEFAULT '',
  store_path TEXT NOT NULL DEFAULT '',
  owner_id VARCHAR(191) NOT NULL DEFAULT '',
  topic_id VARCHAR(191) NOT NULL DEFAULT '',
  size BIGINT NOT NULL DEFAULT 0,
  ext TEXT NOT NULL DEFAULT '',
  private BOOLEAN NOT NULL DEFAULT FALSE,
  external BOOLEAN NOT NULL DEFAULT FALSE,
  tags TEXT NOT NULL DEFAULT '',
  remark TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL
);
"
        .to_string(),
    );

    db.execute(create_users).await?;
    db.execute(create_topics).await?;
    db.execute(create_conversations).await?;
    db.execute(create_topic_members).await?;
    db.execute(create_chat_logs).await?;
    db.execute(create_relations).await?;
    db.execute(create_topic_knocks).await?;
    db.execute(create_auth_tokens).await?;
    db.execute(create_presence_sessions).await?;
    db.execute(create_attachments).await?;

    let add_user_password = Statement::from_string(
        backend,
        "ALTER TABLE users ADD COLUMN password TEXT NOT NULL DEFAULT ''".to_string(),
    );
    let _ = db.execute(add_user_password).await;

    let add_user_enabled = Statement::from_string(
        backend,
        "ALTER TABLE users ADD COLUMN enabled BOOLEAN NOT NULL DEFAULT TRUE".to_string(),
    );
    let _ = db.execute(add_user_enabled).await;

    let add_topic_enabled = Statement::from_string(
        backend,
        "ALTER TABLE topics ADD COLUMN enabled BOOLEAN NOT NULL DEFAULT TRUE".to_string(),
    );
    let _ = db.execute(add_topic_enabled).await;

    let add_presence_endpoint = Statement::from_string(
        backend,
        "ALTER TABLE presence_sessions ADD COLUMN endpoint VARCHAR(191) NOT NULL DEFAULT ''"
            .to_string(),
    );
    let _ = db.execute(add_presence_endpoint).await;

    if matches!(backend, DbBackend::Sqlite | DbBackend::MySql) {
        let idx = Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_conversations_owner_updated ON conversations(owner_id, updated_at);"
                .to_string(),
        );
        db.execute(idx).await?;

        let idx_logs = Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_chat_logs_topic_seq ON chat_logs(topic_id, seq DESC);"
                .to_string(),
        );
        db.execute(idx_logs).await?;

        let idx_member = Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_topic_members_user ON topic_members(user_id);"
                .to_string(),
        );
        db.execute(idx_member).await?;

        let idx_tokens = Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_auth_tokens_user ON auth_tokens(user_id);".to_string(),
        );
        db.execute(idx_tokens).await?;

        let idx_presence = Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_presence_node_updated ON presence_sessions(node_id, updated_at_unix);"
                .to_string(),
        );
        db.execute(idx_presence).await?;

        let idx_attachment_owner = Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_attachments_owner ON attachments(owner_id);"
                .to_string(),
        );
        db.execute(idx_attachment_owner).await?;
    }

    Ok(())
}
