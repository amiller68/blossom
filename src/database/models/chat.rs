use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::database::types::DId;
use crate::database::DatabaseConnection;

/*
CREATE TABLE chats (
  id TEXT PRIMARY KEY DEFAULT (LOWER(HEX(RANDOMBLOB(4))) || '-' ||
         LOWER(HEX(RANDOMBLOB(2))) || '-4' ||
         SUBSTR(LOWER(HEX(RANDOMBLOB(2))), 2) || '-' ||
         SUBSTR('89ab', RANDOM() % 4 + 1, 1) ||
         SUBSTR(LOWER(HEX(RANDOMBLOB(2))), 2) || '-' ||
        LOWER(HEX(RANDOMBLOB(6)))) NOT NULL,
  name TEXT NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
);
*/

#[derive(FromRow, Debug)]
pub struct Chat {
    id: DId,
    name: String,
    created_at: OffsetDateTime,
}

impl Chat {
    pub async fn create(name: &str, conn: &mut DatabaseConnection) -> Result<Uuid, sqlx::Error> {
        let chat_id = sqlx::query_scalar!(
            r#"
            INSERT INTO chats (name, created_at)
            VALUES ($1, CURRENT_TIMESTAMP)
            RETURNING id as 'id: DId'"#,
            name
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(*chat_id.to_owned())
    }

    pub async fn read_all(conn: &mut DatabaseConnection) -> Result<Vec<Chat>, sqlx::Error> {
        let chats = sqlx::query_as!(
            Chat,
            r#"
            SELECT id as "id: DId", name, created_at FROM chats
            "#
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(chats)
    }

    pub async fn read(id: Uuid, conn: &mut DatabaseConnection) -> Result<Chat, sqlx::Error> {
        let d_id: DId = id.into();
        let chat = sqlx::query_as!(
            Chat,
            r#"
            SELECT id as "id: DId", name, created_at FROM chats
            WHERE id = $1
            "#,
            d_id
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(chat)
    }

    pub async fn read_by_name(
        name: &str,
        conn: &mut DatabaseConnection,
    ) -> Result<Chat, sqlx::Error> {
        let chat = sqlx::query_as!(
            Chat,
            r#"
            SELECT id as "id: DId", name, created_at FROM chats
            WHERE name = $1
            "#,
            name
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(chat)
    }

    pub fn id(&self) -> Uuid {
        *self.id.to_owned()
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn created_at(&self) -> OffsetDateTime {
        self.created_at
    }
}

#[cfg(test)]
mod test {
    use crate::tests::prelude::*;

    use super::*;

    #[tokio::test]
    async fn test_create_read() {
        let db_pool = test_database().await;
        let mut conn = db_pool
            .acquire()
            .await
            .expect("Failed to acquire a connection");

        let id = Chat::create("test_chat", &mut conn).await.unwrap();
        let chat = Chat::read(id, &mut conn).await.unwrap();
        assert_eq!(chat.name(), "test_chat");
    }
}
