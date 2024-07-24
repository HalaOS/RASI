use std::{
    ffi::{CStr, CString},
    io::{Error, ErrorKind, Result},
    os::raw::c_void,
    ptr::{null, null_mut},
    slice::from_raw_parts,
    str::{from_utf8_unchecked, FromStr},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::Poll,
};

use rasi::rdbc::*;
use sqlite3_sys as ffi;

struct Sqlite3Driver;

unsafe fn db_error(db: *mut ffi::sqlite3) -> Error {
    Error::new(
        ErrorKind::Other,
        format!(
            "sqlite3: code={}, error={}",
            ffi::sqlite3_errcode(db),
            from_utf8_unchecked(CStr::from_ptr(ffi::sqlite3_errmsg(db)).to_bytes())
        ),
    )
}

impl syscall::Driver for Sqlite3Driver {
    fn create_connection(
        &self,
        driver_name: &str,
        source_name: &str,
    ) -> std::io::Result<Connection> {
        let mut db = null_mut();

        unsafe {
            let rc = ffi::sqlite3_open_v2(
                CString::new(source_name)?.as_ptr(),
                &mut db,
                ffi::SQLITE_OPEN_CREATE
                    | ffi::SQLITE_OPEN_READWRITE
                    | ffi::SQLITE_OPEN_URI
                    | ffi::SQLITE_OPEN_FULLMUTEX,
                null_mut(),
            );

            if rc != ffi::SQLITE_OK {
                return Err(db_error(db));
            }
        }

        let conn = Sqlite3Conn(Arc::new(RawConn(db)));

        Ok((driver_name.to_owned(), conn).into())
    }
}

struct RawConn(*mut ffi::sqlite3);

unsafe impl Send for RawConn {}
unsafe impl Sync for RawConn {}

impl Drop for RawConn {
    fn drop(&mut self) {
        unsafe {
            ffi::sqlite3_close(self.0);
        }
    }
}

struct RawStmt(*mut ffi::sqlite3_stmt);

unsafe impl Send for RawStmt {}
unsafe impl Sync for RawStmt {}

impl Drop for RawStmt {
    fn drop(&mut self) {
        unsafe {
            ffi::sqlite3_finalize(self.0);
        }
    }
}

struct Sqlite3Conn(Arc<RawConn>);

fn exec(conn: &RawConn, sql: &CStr) -> Result<()> {
    unsafe {
        let rc = ffi::sqlite3_exec(conn.0, sql.as_ptr(), None, null_mut(), null_mut());

        if rc != ffi::SQLITE_OK {
            return Err(db_error(conn.0));
        }
    }

    Ok(())
}

fn prepare(conn: Arc<RawConn>, sql: &CStr) -> Result<Prepare> {
    let mut c_stmt = null_mut();

    unsafe {
        let rc = ffi::sqlite3_prepare_v2(conn.0, sql.as_ptr(), -1, &mut c_stmt, null_mut());

        if rc != ffi::SQLITE_OK {
            return Err(db_error(conn.0));
        }
    }

    Ok(Sqlite3Prepare {
        conn,
        stmt: Arc::new(RawStmt(c_stmt)),
    }
    .into())
}

impl syscall::DriverConn for Sqlite3Conn {
    fn poll_ready(&self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn begin(&self) -> std::io::Result<Transaction> {
        exec(&self.0, c"BEGIN;")?;

        Ok(Sqlite3Tx(self.0.clone(), AtomicBool::new(false)).into())
    }

    fn prepare(&self, query: &str) -> std::io::Result<Prepare> {
        prepare(self.0.clone(), CString::new(query)?.as_ref())
    }

    fn exec(&self, query: &str, params: &[SqlParameter<'_>]) -> std::io::Result<Update> {
        self.prepare(query)?.as_driver_query().exec(params)
    }

    fn query(&self, query: &str, params: &[SqlParameter<'_>]) -> std::io::Result<Query> {
        self.prepare(query)?.as_driver_query().query(params)
    }
}

struct Sqlite3Tx(Arc<RawConn>, AtomicBool);

impl Drop for Sqlite3Tx {
    fn drop(&mut self) {
        if self
            .1
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
            .is_ok()
        {
            if let Err(err) = exec(&self.0, c"COMMIT;") {
                log::error!(target:"Sqlite3Tx","auto commit failed, {}",err);
            }
        }
    }
}

impl syscall::DriverTx for Sqlite3Tx {
    fn poll_ready(&self, _cx: &mut std::task::Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_rollback(&self, _cx: &mut std::task::Context<'_>) -> Poll<Result<()>> {
        if self
            .1
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
            .is_ok()
        {
            exec(&self.0, c"ROLLBACK;")?;

            Poll::Ready(Ok(()))
        } else {
            Poll::Ready(Err(Error::new(ErrorKind::Other, "Call rollback twice")))
        }
    }

    fn prepare(&self, query: &str) -> Result<Prepare> {
        prepare(self.0.clone(), CString::new(query)?.as_ref())
    }

    fn exec(&self, query: &str, params: &[SqlParameter<'_>]) -> Result<Update> {
        self.prepare(query)?.as_driver_query().exec(params)
    }

    fn query(&self, query: &str, params: &[SqlParameter<'_>]) -> Result<Query> {
        self.prepare(query)?.as_driver_query().query(params)
    }
}

struct Sqlite3Prepare {
    conn: Arc<RawConn>,
    stmt: Arc<RawStmt>,
}

impl syscall::DriverPrepare for Sqlite3Prepare {
    fn poll_ready(&self, _cx: &mut std::task::Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn exec(&self, params: &[SqlParameter<'_>]) -> Result<Update> {
        self.bind_params(params)?;

        let rc = unsafe { ffi::sqlite3_step(self.stmt.0) };

        match rc {
            ffi::SQLITE_DONE => {
                let last_insert_id = unsafe { ffi::sqlite3_last_insert_rowid(self.conn.0) } as i64;
                let raws_affected = unsafe { ffi::sqlite3_changes(self.conn.0) } as i64;

                return Ok(Sqlite3Update(last_insert_id, raws_affected).into());
            }
            ffi::SQLITE_ROW => {
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    "Call exec on query statement.",
                ))
            }
            _ => return Err(unsafe { db_error(self.conn.0) }),
        }
    }

    fn query(&self, params: &[SqlParameter<'_>]) -> Result<Query> {
        self.bind_params(params)?;

        Ok(Sqlite3Query {
            conn: self.conn.clone(),
            stmt: self.stmt.clone(),
        }
        .into())
    }
}

impl Sqlite3Prepare {
    fn bind_params(&self, params: &[SqlParameter]) -> Result<()> {
        unsafe {
            if ffi::SQLITE_OK != ffi::sqlite3_reset(self.stmt.0) {
                return Err(db_error(self.conn.0));
            }
        }

        let mut named_params = 0;

        for (index, param) in params.iter().enumerate() {
            let (index, value) = match param {
                SqlParameter::Named(name, value) => unsafe {
                    let index = ffi::sqlite3_bind_parameter_index(
                        self.stmt.0,
                        CString::new(name.as_ref())?.as_ptr(),
                    );

                    if index == 0 {
                        return Err(Error::new(
                            ErrorKind::NotFound,
                            format!("no matching parameter is found: {}", name),
                        ));
                    }

                    named_params += 1;

                    (index, value)
                },
                SqlParameter::Offset(value) => (index as i32 + 1 - named_params, value),
            };

            let rc = match value {
                SqlValue::Bool(value) => {
                    let value = if *value { 1 } else { 0 };

                    unsafe { ffi::sqlite3_bind_int(self.stmt.0, index, value) }
                }
                SqlValue::Int(value) => unsafe {
                    ffi::sqlite3_bind_int64(self.stmt.0, index, *value)
                },
                SqlValue::BigInt(value) => unsafe {
                    let value = CString::new(format!("{value}"))?.as_ptr();

                    ffi::sqlite3_bind_text(
                        self.stmt.0,
                        index,
                        value,
                        -1,
                        Some(std::mem::transmute(-1isize)),
                    )
                },
                SqlValue::Float(value) => unsafe {
                    ffi::sqlite3_bind_double(self.stmt.0, index, *value)
                },

                SqlValue::Decimal(value) => unsafe {
                    let value = CString::new(format!("{value}"))?.as_ptr();

                    ffi::sqlite3_bind_text(
                        self.stmt.0,
                        index,
                        value,
                        -1,
                        Some(std::mem::transmute(-1isize)),
                    )
                },
                SqlValue::Binary(value) => unsafe {
                    ffi::sqlite3_bind_blob(
                        self.stmt.0,
                        index,
                        value.as_ptr() as *const c_void,
                        value.len() as i32,
                        Some(std::mem::transmute(-1isize)),
                    )
                },
                SqlValue::String(value) => unsafe {
                    ffi::sqlite3_bind_text(
                        self.stmt.0,
                        index,
                        CString::new(value.as_ref())?.as_ptr(),
                        -1,
                        Some(std::mem::transmute(-1isize)),
                    )
                },
                SqlValue::Null => unsafe { ffi::sqlite3_bind_null(self.stmt.0, index) },
            };

            if rc != ffi::SQLITE_OK {
                return Err(unsafe { db_error(self.conn.0) });
            }
        }

        Ok(())
    }
}

struct Sqlite3Update(i64, i64);

impl syscall::DriverUpdate for Sqlite3Update {
    fn poll_ready(&self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(i64, i64)>> {
        Poll::Ready(Ok((self.0, self.1)))
    }
}

struct Sqlite3Query {
    conn: Arc<RawConn>,
    stmt: Arc<RawStmt>,
}

impl syscall::DriverQuery for Sqlite3Query {
    fn poll_next(&self, _cx: &mut std::task::Context<'_>) -> Poll<Result<Option<Row>>> {
        unsafe {
            match ffi::sqlite3_step(self.stmt.0) {
                ffi::SQLITE_DONE => Poll::Ready(Ok(None)),

                ffi::SQLITE_ROW => Poll::Ready(Ok(Some(
                    Sqlite3Row {
                        conn: self.conn.clone(),
                        stmt: self.stmt.clone(),
                    }
                    .into(),
                ))),

                _ => Poll::Ready(Err(db_error(self.conn.0))),
            }
        }
    }
}

impl syscall::DriverTableMetadata for Sqlite3Query {
    fn cols(&self) -> Result<usize> {
        let count = unsafe { ffi::sqlite3_column_count(self.stmt.0) };

        Ok(count as usize)
    }

    fn col_name(&self, offset: usize) -> Result<&str> {
        unsafe {
            let name = ffi::sqlite3_column_name(self.stmt.0, offset as i32);

            Ok(from_utf8_unchecked(CStr::from_ptr(name).to_bytes()))
        }
    }

    fn col_type(&self, _offset: usize) -> Result<Option<SqlType>> {
        Ok(None)
    }

    fn col_size(&self, _offset: usize) -> Result<Option<usize>> {
        Ok(None)
    }
}

struct Sqlite3Row {
    #[allow(unused)]
    conn: Arc<RawConn>,
    stmt: Arc<RawStmt>,
}

impl syscall::DriverRow for Sqlite3Row {
    fn get(&self, index: usize, sql_type: &SqlType) -> Result<SqlValue<'static>> {
        let col = index as i32;

        match sql_type {
            SqlType::Bool => unsafe {
                if 1 == ffi::sqlite3_column_int(self.stmt.0, col) {
                    Ok(SqlValue::Bool(true))
                } else {
                    Ok(SqlValue::Bool(false))
                }
            },
            SqlType::Int => unsafe {
                Ok(SqlValue::Int(ffi::sqlite3_column_int64(self.stmt.0, col)))
            },
            SqlType::BigInt => unsafe {
                let data = ffi::sqlite3_column_text(self.stmt.0, col) as *const i8;

                if data != null() {
                    let value = from_utf8_unchecked(CStr::from_ptr(data).to_bytes());

                    Ok(SqlValue::BigInt(value.parse().map_err(|err| {
                        Error::new(
                            ErrorKind::InvalidData,
                            format!(
                                "Convert column value({}) to BigInt with error: {}",
                                value, err
                            ),
                        )
                    })?))
                } else {
                    Ok(SqlValue::Null)
                }
            },
            SqlType::Float => unsafe {
                Ok(SqlValue::Float(ffi::sqlite3_column_double(
                    self.stmt.0,
                    col,
                )))
            },

            SqlType::Decimal => unsafe {
                let data = ffi::sqlite3_column_text(self.stmt.0, col) as *const i8;

                if data != null() {
                    let value = from_utf8_unchecked(CStr::from_ptr(data).to_bytes());

                    Ok(SqlValue::Decimal(BigDecimal::from_str(value).map_err(
                        |err| {
                            Error::new(
                                ErrorKind::InvalidData,
                                format!(
                                    "Convert column value({}) to Decimal with error: {}",
                                    value, err
                                ),
                            )
                        },
                    )?))
                } else {
                    Ok(SqlValue::Null)
                }
            },
            SqlType::Binary => unsafe {
                let len = ffi::sqlite3_column_bytes(self.stmt.0, col);
                let data = ffi::sqlite3_column_blob(self.stmt.0, col) as *const u8;
                let data = from_raw_parts(data, len as usize).to_owned();

                Ok(SqlValue::Binary(data.into()))
            },
            SqlType::String => unsafe {
                let data = ffi::sqlite3_column_text(self.stmt.0, col) as *const i8;

                if data != null() {
                    let value = CStr::from_ptr(data);

                    Ok(SqlValue::String(
                        from_utf8_unchecked(value.to_bytes()).into(),
                    ))
                } else {
                    Ok(SqlValue::Null)
                }
            },
            SqlType::Null => Err(Error::new(
                ErrorKind::InvalidInput,
                "Call result get with SqlType::Null",
            )),
        }
    }
}
/// Register sqlite3 database driver with name `sqlite3`.
pub fn register_sqlite3() {
    register_rdbc_driver("sqlite3", Sqlite3Driver).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[futures_test::test]
    async fn test_sqlite3_spec() {
        register_sqlite3();
        rasi_spec::rdbc::run(|| async { open("sqlite3", "").await.unwrap() }).await;
    }
}
