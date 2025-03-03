use diesel::r2d2;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::sqlite::SqliteConnection; // or PgConnection, MysqlConnection

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>; // Or PgConnection, MysqlConnection
pub type DbConnection = PooledConnection<ConnectionManager<SqliteConnection>>; // Or PgConnection, MysqlConnection

pub fn establish_connection_pool(database_url: String) -> DbPool {
    let manager = ConnectionManager::<SqliteConnection>::new(database_url); // Or PgConnection, MysqlConnection
    Pool::builder()
        .build(manager)
        .expect("Failed to create database connection pool")
}

pub fn get_db_connection(pool: &DbPool) -> Result<DbConnection, r2d2::PoolError> {
    pool.get()
}
