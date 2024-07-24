//! RDBC(Rust DataBase Connectivity): future-based database manipulation operations.
//!

use std::{
    borrow::Cow,
    collections::HashMap,
    io::{self, Result},
    sync::{Arc, OnceLock, RwLock},
    task::{Context, Poll},
};

pub use bigdecimal::BigDecimal;

use futures::{future::poll_fn, Future};

/// Sql parameter type variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SqlType {
    Bool,
    Int,
    BigInt,
    Float,
    Decimal,
    Binary,
    String,
    Null,
}

/// A variant type for sql value
#[derive(Debug, Clone, PartialEq)]
pub enum SqlValue<'a> {
    Bool(bool),
    Int(i64),
    BigInt(i128),
    Float(f64),

    Decimal(BigDecimal),
    Binary(Cow<'a, [u8]>),
    String(Cow<'a, str>),
    Null,
}

/// SQL parameter type used by `exec` and `query` methods.
pub enum SqlParameter<'a> {
    Named(Cow<'a, str>, SqlValue<'a>),
    Offset(SqlValue<'a>),
}

/// A rdbc driver must implement the Driver-* traits in this module.
pub mod syscall {
    use super::*;
    ///  Represents database driver that can be shared between threads, and can therefore implement a connection pool
    pub trait Driver: Send + Sync {
        /// Create a new database connection instance with `source_name`.
        ///
        /// When the object returns, the database connection may not actually be established.
        /// The user needs to manually call the [`is_ready`](Connection::is_ready) method to check the status of the connection
        fn create_connection(&self, driver_name: &str, source_name: &str) -> Result<Connection>;
    }

    /// Represents a database connection created by [`create_connection`](Driver::create_connection) function.
    pub trait DriverConn: Send + Sync {
        /// Try establish a real connection to the database.
        fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<()>>;

        /// Starts a transaction via one connection. The default isolation level is dependent on the driver.
        ///
        /// When the transaction object is returned, the database transaction may not actually be ready.
        /// The user needs to manually call the [`is_ready`](DriverTx::is_ready) method to check the status of the transaction.
        fn begin(&self) -> Result<Transaction>;

        /// Create a prepared statement for execution via the connection.
        fn prepare(&self, query: &str) -> Result<Prepare>;

        /// Execute a query that is expected to update some rows.
        fn exec(&self, query: &str, params: &[SqlParameter<'_>]) -> Result<Update>;

        /// Execute a query that is expected to return a result set, such as a SELECT statement
        fn query(&self, query: &str, params: &[SqlParameter<'_>]) -> Result<Query>;
    }

    /// Trait for implementing database connection pooling strategy.
    pub trait ConnectionPool: Send + Sync {
        /// Get an idle connection by `source_name` from this pool.
        fn get_connection(
            &self,
            driver_name: &str,
            source_name: &str,
        ) -> Result<Option<Connection>>;

        /// Put one idle connection into this pool
        fn idle_connection(
            &self,
            driver_name: &str,
            driver_conn: Box<dyn DriverConn>,
        ) -> Result<()>;
    }

    /// Represents the query row result.
    pub trait DriverRow: Send + Sync {
        /// Returns a single field value of this row with suggest sql value type.
        fn get(&self, index: usize, sql_type: &SqlType) -> Result<SqlValue<'static>>;
    }

    /// Represents a prepare statement, created by `prepare` functions.
    pub trait DriverPrepare: Send + Sync {
        /// poll and wait prepare statement compiled.
        fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<()>>;
        /// Execute a query that is expected to update some rows.
        fn exec(&self, params: &[SqlParameter<'_>]) -> Result<Update>;

        /// Execute a query that is expected to return a result set, such as a SELECT statement
        fn query(&self, params: &[SqlParameter<'_>]) -> Result<Query>;
    }

    /// Represents a update statement, created by `exec` functions.
    pub trait DriverUpdate: Send {
        /// Poll query statement result.
        ///
        /// On success, returns the `last insert id` and `rows affected`.
        fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(i64, i64)>>;
    }

    /// Represents query result table's metadata.
    pub trait DriverTableMetadata: Send + Sync {
        /// Returns the number of columns (fields) in each row of the query result.
        fn cols(&self) -> Result<usize>;

        /// Returns the column name associated with the given column number. Column numbers start at 0.
        fn col_name(&self, offset: usize) -> Result<&str>;

        /// Returns the column number associated with the given column name.
        fn col_offset(&self, col_name: &str) -> Result<usize> {
            for i in 0..self.cols()? {
                if self.col_name(i)? == col_name {
                    return Ok(i);
                }
            }

            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Column with name '{}', not found", col_name),
            ))
        }

        /// Returns the `SqlType` of the table from which the given column was fetched. Column numbers start at 0.
        fn col_type(&self, offset: usize) -> Result<Option<SqlType>>;

        /// Returns the size in bytes of the column associated with the given column number. Column numbers start at 0.
        fn col_size(&self, offset: usize) -> Result<Option<usize>>;
    }

    /// Represents a query statement, created by `query` functions.
    pub trait DriverQuery: DriverTableMetadata {
        /// Fetch next row of the query result.
        fn poll_next(&self, cx: &mut Context<'_>) -> Poll<Result<Option<Row>>>;
    }

    /// Represents a driver-specific `Transaction` object.
    ///
    /// The driver should automatically commit this transaction when object is dropping.
    pub trait DriverTx: Send + Sync {
        /// Poll and check if the transaction is ready.
        fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<()>>;

        /// Rollback this transaction.
        ///
        /// If this function encounters an error, the tx should be considered
        /// to have failed permanently, and no more `DriverTx` methods should be called.
        fn poll_rollback(&self, cx: &mut Context<'_>) -> Poll<Result<()>>;

        /// Create a prepared statement for execution via the connection.
        fn prepare(&self, query: &str) -> Result<Prepare>;

        /// Execute a query that is expected to update some rows.
        fn exec(&self, query: &str, params: &[SqlParameter<'_>]) -> Result<Update>;

        /// Execute a query that is expected to return a result set, such as a SELECT statement
        fn query(&self, query: &str, params: &[SqlParameter<'_>]) -> Result<Query>;
    }
}

/// The connection object that represents a rdbc database connection object.
pub struct Connection(String, Option<Box<dyn syscall::DriverConn>>);

impl<T: syscall::DriverConn + 'static> From<(String, T)> for Connection {
    fn from(value: (String, T)) -> Self {
        Self(value.0, Some(Box::new(value.1)))
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        if let Some(conn_pool) = CONN_POOL.get() {
            if let Err(err) = conn_pool.idle_connection(&self.0, self.1.take().unwrap()) {
                log::error!(
                    target: "RDBC",
                    "Put idle connection int cache with error, {}",
                    err
                );
            }
        }
    }
}

impl Connection {
    /// Get inner [`DriverConn`] reference.
    pub fn as_driver_conn(&self) -> &dyn syscall::DriverConn {
        &*self.1.as_deref().unwrap()
    }

    /// Check if the object is already connected.
    ///
    /// When the `Connection` object is created by [`create_connection`](Driver::create_connection)
    /// function, you should first call this function to waiting
    /// connected event or connection error event.
    pub async fn is_ready(&self) -> Result<()> {
        poll_fn(|cx| self.1.as_ref().unwrap().poll_ready(cx)).await
    }

    /// Start a new `transaction` via this connection.
    ///
    /// This function will automatically call the [`poll_ready`](DriverTx::poll_ready)
    /// function after the transaction object has been created,
    /// so the user should not have to call this function again.
    pub async fn begin(&self) -> Result<Transaction> {
        let tx = self.1.as_ref().unwrap().begin()?;

        poll_fn(|cx| tx.0.poll_ready(cx)).await.map(|_| tx)
    }

    /// Create a prepared statement for execution via the connection.
    pub async fn prepare(&self, query: &str) -> Result<Prepare> {
        let prepare = self.1.as_ref().unwrap().prepare(query)?;

        poll_fn(|cx| prepare.0.poll_ready(cx))
            .await
            .map(|_| prepare)
    }

    /// Execute a query that is expected to update some rows.
    pub async fn exec(&self, query: &str, params: &[SqlParameter<'_>]) -> Result<Update> {
        self.1.as_ref().unwrap().exec(query, params)
    }

    /// Execute a query that is expected to return a result set, such as a SELECT statement
    pub async fn query(&self, query: &str, params: &[SqlParameter<'_>]) -> Result<Query> {
        self.1.as_ref().unwrap().query(query, params)
    }
}

/// The query object that represents a `exec` statement.
pub struct Prepare(Box<dyn syscall::DriverPrepare>);

impl<T: syscall::DriverPrepare + 'static> From<T> for Prepare {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}

impl Prepare {
    /// Get inner [`DriverPrepare`] reference.
    pub fn as_driver_query(&self) -> &dyn syscall::DriverPrepare {
        &*self.0
    }

    /// Execute a query that is expected to update some rows.
    pub async fn exec(&self, params: &[SqlParameter<'_>]) -> Result<Update> {
        self.0.exec(params)
    }

    /// Execute a query that is expected to return a result set, such as a SELECT statement
    pub async fn query(&self, params: &[SqlParameter<'_>]) -> Result<Query> {
        self.0.query(params)
    }
}

/// The update future that represents a `exec` statement.
pub struct Update(Box<dyn syscall::DriverUpdate>);

impl<T: syscall::DriverUpdate + 'static> From<T> for Update {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}

impl Future for Update {
    type Output = Result<(i64, i64)>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.poll_ready(cx)
    }
}

/// The query object that represents a `exec` statement.
pub struct Query(Box<dyn syscall::DriverQuery>);

impl<T: syscall::DriverQuery + 'static> From<T> for Query {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}

impl Query {
    /// Get inner [`DriverQuery`] reference.
    pub fn as_driver_query(&self) -> &dyn syscall::DriverQuery {
        &*self.0
    }

    /// Fetch next row of the query result.
    pub async fn next(&self) -> Result<Option<Row>> {
        poll_fn(|cx| self.0.poll_next(cx)).await
    }

    /// Returns the number of columns (fields) in each row of the query result.
    pub async fn cols(&self) -> Result<usize> {
        self.0.cols()
    }

    /// Returns the column name associated with the given column number. Column numbers start at 0.
    pub async fn col_name(&self, offset: usize) -> Result<&str> {
        self.0.col_name(offset)
    }

    /// Returns the column number associated with the given column name.
    pub async fn col_offset(&self, col_name: &str) -> Result<usize> {
        self.0.col_offset(col_name)
    }

    /// Returns the `SqlType` of the table from which the given column was fetched. Column numbers start at 0.
    pub async fn col_type(&self, offset: usize) -> Result<Option<SqlType>> {
        self.0.col_type(offset)
    }

    /// Returns the size in bytes of the column associated with the given column number. Column numbers start at 0.
    pub async fn col_size(&self, offset: usize) -> Result<Option<usize>> {
        self.0.col_size(offset)
    }
}

/// The structure that represents the query row result.
pub struct Row(Box<dyn syscall::DriverRow>);

impl<T: syscall::DriverRow + 'static> From<T> for Row {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}

impl Row {
    /// Get inner [`DriverRow`] reference.
    pub fn as_driver_row(&self) -> &dyn syscall::DriverRow {
        &*self.0
    }

    /// Returns a single field value of this row with suggest sql value type.
    pub fn get(&self, index: usize, sql_type: &SqlType) -> Result<SqlValue<'static>> {
        self.0.get(index, sql_type)
    }

    /// Returns a single field value of this row as sql type [`bool`](SqlType::Bool).
    ///
    /// Returns error, if the column is null.
    pub fn ensure_bool(&self, index: usize) -> Result<bool> {
        Ok(self.get_bool(index)?.ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "Column value is null",
        ))?)
    }

    /// Returns a single field value of this row as sql type [`bool`](SqlType::Bool)
    pub fn get_bool(&self, index: usize) -> Result<Option<bool>> {
        match self.get(index, &SqlType::Bool)? {
            SqlValue::Bool(value) => Ok(Some(value)),
            SqlValue::Null => Ok(None),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Can't convert column value to bool type.",
            )),
        }
    }

    /// Returns a single field value of this row as sql type [`int`](SqlType::Int).
    ///
    /// Returns error, if the column is null.
    pub fn ensure_int(&self, index: usize) -> Result<i64> {
        Ok(self.get_int(index)?.ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "Column value is null",
        ))?)
    }

    /// Returns a single field value of this row as sql type [`int`](SqlType::Int)
    pub fn get_int(&self, index: usize) -> Result<Option<i64>> {
        match self.get(index, &SqlType::Int)? {
            SqlValue::Int(value) => Ok(Some(value)),
            SqlValue::Null => Ok(None),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Can't convert column value to bool type.",
            )),
        }
    }

    /// Returns a single field value of this row as sql type [`bigint`](SqlType::BigInt).
    ///
    /// Returns error, if the column is null.
    pub fn ensure_bigint(&self, index: usize) -> Result<i128> {
        Ok(self.get_bigint(index)?.ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "Column value is null",
        ))?)
    }

    /// Returns a single field value of this row as sql type [`bigint`](SqlType::BigInt)
    pub fn get_bigint(&self, index: usize) -> Result<Option<i128>> {
        match self.get(index, &SqlType::BigInt)? {
            SqlValue::BigInt(value) => Ok(Some(value)),
            SqlValue::Null => Ok(None),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Can't convert column value to bigint type.",
            )),
        }
    }

    /// Returns a single field value of this row as sql type [`decimal`](SqlType::Decimal).
    ///
    /// Returns error, if the column is null.
    pub fn ensure_decimal(&self, index: usize) -> Result<BigDecimal> {
        Ok(self.get_decimal(index)?.ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "Column value is null",
        ))?)
    }

    /// Returns a single field value of this row as sql type [`decimal`](SqlType::Decimal)
    pub fn get_decimal(&self, index: usize) -> Result<Option<BigDecimal>> {
        match self.get(index, &SqlType::Decimal)? {
            SqlValue::Decimal(value) => Ok(Some(value)),
            SqlValue::Null => Ok(None),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Can't convert column value to decimal type.",
            )),
        }
    }

    /// Returns a single field value of this row as sql type [`float`](SqlType::Float).
    ///
    /// Returns error, if the column is null.
    pub fn ensure_float(&self, index: usize) -> Result<f64> {
        Ok(self.get_float(index)?.ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "Column value is null",
        ))?)
    }

    /// Returns a single field value of this row as sql type [`float`](SqlType::Float)
    pub fn get_float(&self, index: usize) -> Result<Option<f64>> {
        match self.get(index, &SqlType::Float)? {
            SqlValue::Float(value) => Ok(Some(value)),
            SqlValue::Null => Ok(None),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Can't convert column value to decimal type.",
            )),
        }
    }

    /// Returns a single field value of this row as sql type [`binary`](SqlType::Binary).
    ///
    /// Returns error, if the column is null.
    pub fn ensure_binary(&self, index: usize) -> Result<Vec<u8>> {
        Ok(self.get_binary(index)?.ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "Column value is null",
        ))?)
    }

    /// Returns a single field value of this row as sql type [`binary`](SqlType::Binary)
    pub fn get_binary(&self, index: usize) -> Result<Option<Vec<u8>>> {
        match self.get(index, &SqlType::Binary)? {
            SqlValue::Binary(value) => Ok(Some(value.into_owned())),
            SqlValue::Null => Ok(None),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Can't convert column value to decimal type.",
            )),
        }
    }

    /// Returns a single field value of this row as sql type [`string`](SqlType::String).
    ///
    /// Returns error, if the column is null.
    pub fn ensure_string(&self, index: usize) -> Result<String> {
        Ok(self.get_string(index)?.ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "Column value is null",
        ))?)
    }

    /// Returns a single field value of this row as sql type [`string`](SqlType::String)
    pub fn get_string(&self, index: usize) -> Result<Option<String>> {
        match self.get(index, &SqlType::String)? {
            SqlValue::String(value) => Ok(Some(value.into_owned())),
            SqlValue::Null => Ok(None),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Can't convert column value to decimal type.",
            )),
        }
    }
}

/// The connection object that represents a rdbc database transaction object.
pub struct Transaction(Box<dyn syscall::DriverTx>);

impl<T: syscall::DriverTx + 'static> From<T> for Transaction {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}

impl Transaction {
    /// Get inner [`DriverConn`] reference.
    pub fn as_driver_tx(&self) -> &dyn syscall::DriverTx {
        &*self.0
    }

    /// Rollback this transaction.
    /// see [`poll_rollback`](DriverTx::poll_rollback) for details.
    pub async fn rollback(&self) -> Result<()> {
        poll_fn(|cx: &mut Context| self.0.poll_rollback(cx)).await
    }

    /// Create a prepared statement for execution via the connection.
    pub async fn prepare(&self, query: &str) -> Result<Prepare> {
        let prepare = self.0.prepare(query)?;

        poll_fn(|cx| prepare.0.poll_ready(cx))
            .await
            .map(|_| prepare)
    }

    /// Execute a query that is expected to update some rows.
    pub async fn exec(&self, query: &str, params: &[SqlParameter<'_>]) -> Result<Update> {
        self.0.exec(query, params)
    }

    /// Execute a query that is expected to return a result set, such as a SELECT statement
    pub async fn query(&self, query: &str, params: &[SqlParameter<'_>]) -> Result<Query> {
        self.0.query(query, params)
    }
}

#[derive(Default)]
struct GlobalRegister {
    drivers: RwLock<HashMap<String, Arc<Box<dyn syscall::Driver>>>>,
}

static REGISTER: OnceLock<GlobalRegister> = OnceLock::new();
static CONN_POOL: OnceLock<Box<dyn syscall::ConnectionPool>> = OnceLock::new();

fn get_register() -> &'static GlobalRegister {
    REGISTER.get_or_init(|| Default::default())
}

/// Register a connection reused strategy to the global context of this application.
pub fn register_pool_strategy<P: syscall::ConnectionPool + 'static>(conn_pool: P) {
    if CONN_POOL.set(Box::new(conn_pool)).is_err() {
        panic!("Call register_pool_strategy more than once.")
    }
}

/// Opens a database connection specified by the database driver name and a driver-specific
/// data source name, usually consisting of at least a database name and connection information.
///
/// # Panics
///
/// this function will panic, if the driver with the supplied `driver_name` is not registered.
pub async fn open<D: AsRef<str>, S: AsRef<str>>(
    driver_name: D,
    source_name: S,
) -> Result<Connection> {
    let drivers = get_register()
        .drivers
        .read()
        .map_err(|err| io::Error::new(io::ErrorKind::Interrupted, err.to_string()))?;

    let driver_name = driver_name.as_ref();
    let source_name = source_name.as_ref();

    if let Some(database) = drivers.get(driver_name) {
        // try get connection from conn_pool first.
        if let Some(conn_pool) = CONN_POOL.get() {
            log::debug!(target: "RDBC", "Try get connection from cached pool");

            if let Some(conn) = conn_pool.get_connection(driver_name, source_name)? {
                log::debug!(
                    target: "RDBC",
                    "Get connection from cached pool, driver={}, source={}",
                    driver_name,source_name,
                );
                return Ok(conn);
            }
        }

        log::debug!(
            target: "RDBC",
            "Create new connection, driver={}, source={}",
            driver_name,
            source_name
        );

        let conn = database.create_connection(driver_name, source_name)?;

        poll_fn(|cx| conn.1.as_ref().unwrap().poll_ready(cx)).await?;

        return Ok(conn);
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Unknown driver: {}", driver_name),
        ));
    }
}

/// Register a new database driver to the global context of this application.
///
/// # Multiple drivers.
///
/// Register multiple drivers to the same global context of this application,
/// allowed only if these drivers have different names.
///
/// # Panics
///
/// If register two driver with the same `name`, this function will panics.
pub fn register_rdbc_driver<N: AsRef<str>, D: syscall::Driver + 'static>(
    driver_name: N,
    database: D,
) -> Result<()> {
    let mut drivers = get_register()
        .drivers
        .write()
        .map_err(|err| io::Error::new(io::ErrorKind::Interrupted, err.to_string()))?;

    assert!(
        drivers
            .insert(
                driver_name.as_ref().to_owned(),
                Arc::new(Box::new(database))
            )
            .is_none(),
        "register driver twice: {}",
        driver_name.as_ref(),
    );

    Ok(())
}
