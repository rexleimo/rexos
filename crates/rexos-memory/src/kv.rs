use rusqlite::{OptionalExtension, Transaction, TransactionBehavior};

use crate::MemoryStore;

impl MemoryStore {
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
            .query_row("SELECT value FROM kv WHERE key=?1", (key,), |row| {
                row.get(0)
            })
            .optional()?;
        Ok(value)
    }

    pub fn kv_update<F>(&self, key: &str, f: F) -> anyhow::Result<Option<String>>
    where
        F: FnOnce(Option<String>) -> anyhow::Result<Option<String>>,
    {
        let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)?;
        let current = tx
            .query_row("SELECT value FROM kv WHERE key=?1", (key,), |row| {
                row.get(0)
            })
            .optional()?;

        let next = f(current)?;
        match &next {
            Some(value) => {
                tx.execute(
                    "INSERT INTO kv (key, value) VALUES (?1, ?2)\n            ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                    (key, value),
                )?;
            }
            None => {
                tx.execute("DELETE FROM kv WHERE key=?1", (key,))?;
            }
        }

        tx.commit()?;
        Ok(next)
    }
}
