use std::path::Path;

use anyhow::Context;
use rusqlite::{Connection, OptionalExtension};

use crate::llm::openai_compat::{ChatMessage, Role, ToolCall};
use crate::paths::RexosPaths;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub id: i64,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    pub name: Option<String>,
    pub tool_call_id: Option<String>,
    pub tool_calls_json: Option<String>,
}

#[derive(Debug)]
pub struct MemoryStore {
    conn: Connection,
}

impl MemoryStore {
    pub fn open_or_create(paths: &RexosPaths) -> anyhow::Result<Self> {
        Self::open_or_create_at_path(&paths.db_path())
    }

    fn open_or_create_at_path(path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("open sqlite db: {}", path.display()))?;

        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> anyhow::Result<()> {
        self.conn.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;

            CREATE TABLE IF NOT EXISTS kv (
              key TEXT PRIMARY KEY,
              value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sessions (
              session_id TEXT PRIMARY KEY,
              created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS messages (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              session_id TEXT NOT NULL,
              role TEXT NOT NULL,
              content TEXT NOT NULL,
              created_at TEXT NOT NULL,
              name TEXT,
              tool_call_id TEXT,
              tool_calls_json TEXT,
              FOREIGN KEY (session_id) REFERENCES sessions(session_id)
            );
            CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id);
            "#,
        )?;

        // Backfill schema for existing databases created with earlier versions.
        let _ = self
            .conn
            .execute("ALTER TABLE messages ADD COLUMN name TEXT", ());
        let _ = self
            .conn
            .execute("ALTER TABLE messages ADD COLUMN tool_call_id TEXT", ());
        let _ = self
            .conn
            .execute("ALTER TABLE messages ADD COLUMN tool_calls_json TEXT", ());

        Ok(())
    }

    pub fn kv_set(&self, key: &str, value: &str) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO kv (key, value) VALUES (?1, ?2)\n            ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            (key, value),
        )?;
        Ok(())
    }

    pub fn kv_get(&self, key: &str) -> anyhow::Result<Option<String>> {
        let value = self
            .conn
            .query_row("SELECT value FROM kv WHERE key=?1", (key,), |row| row.get(0))
            .optional()?;
        Ok(value)
    }

    pub fn append_message(&self, session_id: &str, role: &str, content: &str) -> anyhow::Result<()> {
        let now = now_epoch_seconds().to_string();

        self.conn.execute(
            "INSERT INTO sessions (session_id, created_at) VALUES (?1, ?2)\n            ON CONFLICT(session_id) DO NOTHING",
            (session_id, &now),
        )?;

        self.conn.execute(
            "INSERT INTO messages (session_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            (session_id, role, content, &now),
        )?;

        Ok(())
    }

    pub fn list_messages(&self, session_id: &str) -> anyhow::Result<Vec<Message>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, role, content, created_at, name, tool_call_id, tool_calls_json FROM messages WHERE session_id=?1 ORDER BY id ASC",
        )?;

        let mut rows = stmt.query((session_id,))?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(Message {
                id: row.get(0)?,
                session_id: session_id.to_string(),
                role: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
                name: row.get(4)?,
                tool_call_id: row.get(5)?,
                tool_calls_json: row.get(6)?,
            });
        }
        Ok(out)
    }

    pub fn append_chat_message(&self, session_id: &str, msg: &ChatMessage) -> anyhow::Result<()> {
        let now = now_epoch_seconds().to_string();

        self.conn.execute(
            "INSERT INTO sessions (session_id, created_at) VALUES (?1, ?2)\n            ON CONFLICT(session_id) DO NOTHING",
            (session_id, &now),
        )?;

        let role = role_to_str(msg.role);
        let content = msg.content.clone().unwrap_or_default();
        let tool_calls_json = msg
            .tool_calls
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        self.conn.execute(
            "INSERT INTO messages (session_id, role, content, created_at, name, tool_call_id, tool_calls_json)\n            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (
                session_id,
                role,
                content,
                &now,
                msg.name.as_deref(),
                msg.tool_call_id.as_deref(),
                tool_calls_json.as_deref(),
            ),
        )?;

        Ok(())
    }

    pub fn list_chat_messages(&self, session_id: &str) -> anyhow::Result<Vec<ChatMessage>> {
        let msgs = self.list_messages(session_id)?;
        let mut out = Vec::with_capacity(msgs.len());

        for m in msgs {
            let role = role_from_str(&m.role)?;
            let tool_calls = match m.tool_calls_json.as_deref() {
                Some(s) if !s.trim().is_empty() => Some(serde_json::from_str::<Vec<ToolCall>>(s)?),
                _ => None,
            };

            let content = if m.content.is_empty() && tool_calls.is_some() {
                None
            } else {
                Some(m.content)
            };

            out.push(ChatMessage {
                role,
                content,
                name: m.name,
                tool_call_id: m.tool_call_id,
                tool_calls,
            });
        }

        Ok(out)
    }
}

fn role_to_str(role: Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    }
}

fn role_from_str(s: &str) -> anyhow::Result<Role> {
    match s {
        "system" => Ok(Role::System),
        "user" => Ok(Role::User),
        "assistant" => Ok(Role::Assistant),
        "tool" => Ok(Role::Tool),
        _ => anyhow::bail!("unknown role: {s}"),
    }
}

fn now_epoch_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn kv_round_trip() {
        let tmp = tempdir().unwrap();
        let db_path = tmp.path().join("test.db");
        let store = MemoryStore::open_or_create_at_path(&db_path).unwrap();

        assert_eq!(store.kv_get("missing").unwrap(), None);
        store.kv_set("a", "1").unwrap();
        assert_eq!(store.kv_get("a").unwrap(), Some("1".to_string()));
        store.kv_set("a", "2").unwrap();
        assert_eq!(store.kv_get("a").unwrap(), Some("2".to_string()));
    }

    #[test]
    fn messages_persist_across_reopen() {
        let tmp = tempdir().unwrap();
        let db_path = tmp.path().join("test.db");

        {
            let store = MemoryStore::open_or_create_at_path(&db_path).unwrap();
            store.append_message("s1", "user", "hello").unwrap();
            store.append_message("s1", "assistant", "world").unwrap();
        }

        let store = MemoryStore::open_or_create_at_path(&db_path).unwrap();
        let msgs = store.list_messages("s1").unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[0].content, "hello");
        assert_eq!(msgs[1].role, "assistant");
        assert_eq!(msgs[1].content, "world");
    }

    #[test]
    fn tool_calls_round_trip() {
        let tmp = tempdir().unwrap();
        let db_path = tmp.path().join("test.db");
        let store = MemoryStore::open_or_create_at_path(&db_path).unwrap();

        let assistant = ChatMessage {
            role: Role::Assistant,
            content: None,
            name: None,
            tool_call_id: None,
            tool_calls: Some(vec![ToolCall {
                id: "call_1".to_string(),
                kind: "function".to_string(),
                function: crate::llm::openai_compat::ToolFunction {
                    name: "fs_read".to_string(),
                    arguments: "{\"path\":\"README.md\"}".to_string(),
                },
            }]),
        };
        store.append_chat_message("s1", &assistant).unwrap();

        let tool = ChatMessage {
            role: Role::Tool,
            content: Some("file contents".to_string()),
            name: None,
            tool_call_id: Some("call_1".to_string()),
            tool_calls: None,
        };
        store.append_chat_message("s1", &tool).unwrap();

        let msgs = store.list_chat_messages("s1").unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, Role::Assistant);
        assert_eq!(msgs[0].content, None);
        assert_eq!(msgs[0].tool_calls.as_ref().unwrap()[0].id, "call_1");
        assert_eq!(msgs[1].role, Role::Tool);
        assert_eq!(msgs[1].tool_call_id.as_deref(), Some("call_1"));
        assert_eq!(msgs[1].content.as_deref(), Some("file contents"));
    }
}
