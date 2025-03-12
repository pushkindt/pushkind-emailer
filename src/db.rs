use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::sqlite::SqliteConnection;
use log::error;

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>; // Or PgConnection, MysqlConnection
pub type DbConnection = PooledConnection<ConnectionManager<SqliteConnection>>; // Or PgConnection, MysqlConnection

pub fn establish_connection_pool(database_url: String) -> Result<DbPool, String> {
    let manager = ConnectionManager::<SqliteConnection>::new(database_url); // Or PgConnection, MysqlConnection
    Pool::builder()
        .build(manager)
        .map_err(|e| format!("Cannot build a DB Pool: {e}"))
}

pub fn get_db_connection(pool: &DbPool) -> Option<DbConnection> {
    match pool.get() {
        Ok(conn) => Some(conn),
        Err(err) => {
            error!("Database connection error: {}", err);
            None
        }
    }
}
