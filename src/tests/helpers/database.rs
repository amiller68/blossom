use crate::database::Database;
use url::Url;

pub(crate) async fn test_database() -> Database {
    Database::connect(&Url::parse("sqlite::memory:").unwrap())
        .await
        .unwrap()
}
