use lazy_static::lazy_static;
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

// Define migrations. These are applied atomically.
lazy_static! {
    static ref MIGRATIONS: Migrations<'static> =
        Migrations::new(vec![
            M::up(include_str!("../migrations/00001_initial.sql")),

            // PRAGMA are better applied outside of migrations, see below for details.
            // M::up(r#"
            //       ALTER TABLE friend ADD COLUMN birthday TEXT;
            //       ALTER TABLE friend ADD COLUMN comment TEXT;
            //       "#),

            // // This migration can be reverted
            // M::up("CREATE TABLE animal(name TEXT);")
            // .down("DROP TABLE animal;")

            // In the future, if the need to change the schema arises, put
            // migrations here, like so:
            // M::up("CREATE INDEX UX_friend_email ON friend(email);"),
            // M::up("CREATE INDEX UX_friend_name ON friend(name);"),
        ]);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn do_migrate(conn: &mut Connection) -> crate::Result<()> {
    // Update the database schema, atomically
    MIGRATIONS
        .to_latest(conn)
        .map_err(|e| crate::ClientError::DbMigrateError(e.to_string()))
}

#[test]
fn migrations_test() {
    let r = MIGRATIONS.validate();
    if r.is_err() {
        println!("migrate validate fail {:?}", r);
    }
    assert!(r.is_ok());
}
