-- configs 
CREATE TABLE IF NOT EXISTS "configs" (
    `id` integer,
    `key` text UNIQUE,
    `value` text,
    PRIMARY KEY (`id`)
);

--- 
--- users 
--- 
CREATE TABLE IF NOT EXISTS "users" (
    `user_id` text UNIQUE,
    `name` text,
    `avatar` text,
    `public_key` text,
    `remark` text,
    `is_contact` integer,
    `is_star` integer,
    `is_blocked` integer,
    `locale` text,
    `city` text,
    `country` text,
    `source` text,
    `created_at` datetime,
    `cached_at` datetime,
    PRIMARY KEY (`user_id`)
);

CREATE INDEX IF NOT EXISTS `idx_users_cached_at` ON `users`(`cached_at`);

--- 
--- conversations 
--- 
CREATE TABLE IF NOT EXISTS "conversations" (
    `topic_id` text UNIQUE,
    `owner_id` text,
    `last_seq` integer,
    `last_read_seq` integer,
    `multiple` integer,
    `attendee` text,
    `name` text,
    `icon` text,
    `sticky` integer,
    `mute` integer,
    `source` text,
    `last_sender_id` text,
    `last_message` text,
    `last_message_at` datetime,
    `cached_at` datetime,
    PRIMARY KEY (`topic_id`)
);

CREATE INDEX IF NOT EXISTS `idx_conversations_cached_at` ON `conversations`(`cached_at`);

CREATE INDEX IF NOT EXISTS `idx_conversations_sticky` ON `conversations`(`sticky`);

--- 
--- topics 
--- 
CREATE TABLE IF NOT EXISTS "topics" (
    `id` text UNIQUE,
    `name` text,
    `icon` text,
    `remark` text,
    `owner_id` text,
    `attendee_id` text,
    `admins` text,
    `members` integer,
    `last_seq` integer,
    `multiple` integer,
    `source` text,
    `private` integer,
    `notice` text,
    `silent` integer,
    `cached_at` datetime,
    `created_at` datetime,
    PRIMARY KEY (`id`)
);

CREATE INDEX IF NOT EXISTS `idx_topics_cached_at` ON `topics`(`cached_at`);

CREATE INDEX IF NOT EXISTS `idx_topics_multiple` ON `topics`(`multiple`);

--- 
--- topic_members 
--- 
CREATE TABLE IF NOT EXISTS "topic_members" (
    `topic_id` text,
    `user_id` text,
    `is_owner` integer,
    `is_admin` integer,
    `remark` text,
    `silent` integer,
    `joined_at` datetime,
    `cached_at` datetime,
    PRIMARY KEY (`topic_id`, `user_id`)
);

CREATE INDEX IF NOT EXISTS `idx_topic_members_cached_at` ON `topic_members`(`cached_at`);

CREATE INDEX IF NOT EXISTS `idx_topic_members_is_admin` ON `topic_members`(`topic_id`, `is_admin`);

--- 
--- messages 
--- 
CREATE TABLE IF NOT EXISTS "messages" (
    `topic_id` text,
    `id` text,
    `seq` integer,
    `created_at` datetime,
    `sender_id` text,
    `content` text,
    `read` integer,
    `recall` integer,
    `status` integer,
    -- See models::ChatLogStatus
    `cached_at` datetime,
    PRIMARY KEY (`topic_id`, `id`)
);

CREATE INDEX IF NOT EXISTS `idx_messages_cached_att` ON `messages`(`cached_at`);

CREATE INDEX IF NOT EXISTS `idx_messages_topic_id_sender_id_seq` ON `messages`(`topic_id`, `sender_id`, `seq`);