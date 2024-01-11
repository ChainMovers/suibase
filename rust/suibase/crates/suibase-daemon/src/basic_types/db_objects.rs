use rusqlite::Connection;

// Generic DBTable.
pub trait DBTable {
    fn create_table(
        conn: &Connection,
        workdir_name: String,        // Always prepended to table_name.
        namespace: Option<String>,   // Optional. Always prepended to table name.
        name_suffix: Option<String>, // Optional. Sometimes appended to table name (ignored with table targeted by foreign key).
    ) -> rusqlite::Result<()>;
}

// Can have its Versioned<T> JSON data persisted in a DB table.
pub trait DBVersionedJSON: DBTable {
    fn update(conn: &Connection) -> rusqlite::Result<()>;
}
