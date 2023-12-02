use rusqlite::Connection;

// Generic DBTable.
pub trait DBTable {
    fn create_table(conn: &Connection) -> rusqlite::Result<()>;
}

// Can have its Versioned<T> JSON data persisted in a DB table.
pub trait DBVersionedJSON: DBTable {
    fn update(conn: &Connection) -> rusqlite::Result<()>;
}
