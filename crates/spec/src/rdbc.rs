use rasi::rdbc::{Connection, SqlParameter, SqlValue};
use std::future::Future;

async fn run_spec<R, F>(name: &str, conn: Connection, f: F)
where
    F: FnOnce(Connection) -> R,
    R: Future<Output = ()> + Send,
{
    print!("rdbcq-spec({})", name);

    f(conn).await;

    println!(" -- ok");
}

macro_rules! spec {
    ($t:expr,$setup: expr) => {
        run_spec(stringify!($t), $setup().await, $t).await
    };
}

/// Run `rdbcq` spec.
pub async fn run<R, S>(setup: S)
where
    S: Fn() -> R,
    R: Future<Output = Connection>,
{
    spec!(crud, setup);
    spec!(tx, setup);
    spec!(prepare, setup);
}

async fn crud(conn: Connection) {
    conn.exec("CREATE TABLE t1(x INT,y INT);", &[])
        .await
        .unwrap();

    conn.exec("INSERT INTO t1(x,y) VALUES(4,4);", &[])
        .await
        .unwrap();

    conn.exec("INSERT INTO t1(x,y) VALUES(5,5);", &[])
        .await
        .unwrap();

    conn.exec("INSERT INTO t1(x,y) VALUES(6,6);", &[])
        .await
        .unwrap();

    let query = conn.query("SELECT count(x) from t1", &[]).await.unwrap();

    let row = query.next().await.unwrap().unwrap();

    assert_eq!(row.ensure_int(0).unwrap(), 3);

    assert!(query.next().await.unwrap().is_none());

    let query = conn.query("SELECT * from t1", &[]).await.unwrap();

    let mut idx = 0;

    while let Some(row) = query.next().await.unwrap() {
        let value = idx + 4;
        idx += 1;

        assert_eq!(row.ensure_int(0).unwrap(), value);
        assert_eq!(row.ensure_int(1).unwrap(), value);
    }

    assert_eq!(idx, 3);

    conn.exec("UPDATE t1 SET y=1;", &[]).await.unwrap();

    let mut idx = 0;

    while let Some(row) = query.next().await.unwrap() {
        let value = idx + 4;
        idx += 1;

        assert_eq!(row.ensure_int(0).unwrap(), value);
        assert_eq!(row.ensure_int(1).unwrap(), 1);
    }

    assert_eq!(idx, 3);

    conn.exec("UPDATE t1 SET y=1;", &[]).await.unwrap();

    conn.exec("DELETE FROM t1;VACUUM;", &[]).await.unwrap();

    let query = conn.query("SELECT count(x) from t1", &[]).await.unwrap();

    let row = query.next().await.unwrap().unwrap();

    assert_eq!(row.ensure_int(0).unwrap(), 0);
}

async fn tx(conn: Connection) {
    conn.exec("CREATE TABLE t1(x INT,y INT);", &[])
        .await
        .unwrap();

    {
        let tx = conn.begin().await.unwrap();

        tx.exec("INSERT INTO t1(x,y) VALUES(4,4);", &[])
            .await
            .unwrap();

        tx.exec("INSERT INTO t1(x,y) VALUES(5,5);", &[])
            .await
            .unwrap();

        tx.exec("INSERT INTO t1(x,y) VALUES(6,6);", &[])
            .await
            .unwrap();
    }

    let query = conn.query("SELECT count(x) from t1", &[]).await.unwrap();

    let row = query.next().await.unwrap().unwrap();

    assert_eq!(row.ensure_int(0).unwrap(), 3);

    conn.exec("DELETE FROM t1;VACUUM;", &[]).await.unwrap();

    {
        let tx = conn.begin().await.unwrap();

        tx.exec("INSERT INTO t1(x,y) VALUES(4,4);", &[])
            .await
            .unwrap();

        tx.exec("INSERT INTO t1(x,y) VALUES(5,5);", &[])
            .await
            .unwrap();

        tx.exec("INSERT INTO t1(x,y) VALUES(6,6);", &[])
            .await
            .unwrap();

        tx.rollback().await.unwrap();
    }

    let query = conn.query("SELECT count(x) from t1", &[]).await.unwrap();

    let row = query.next().await.unwrap().unwrap();

    assert_eq!(row.ensure_int(0).unwrap(), 0);
}

async fn prepare(conn: Connection) {
    conn.exec("CREATE TABLE t1(x INT,y INT);", &[])
        .await
        .unwrap();

    conn.exec("INSERT INTO t1(x,y) VALUES(4,4);", &[])
        .await
        .unwrap();

    conn.exec("INSERT INTO t1(x,y) VALUES(5,5);", &[])
        .await
        .unwrap();

    conn.exec("INSERT INTO t1(x,y) VALUES(6,6);", &[])
        .await
        .unwrap();

    let prepare = conn
        .prepare("SELECT count(*) FROM t1 where x > ?")
        .await
        .unwrap();

    let query = prepare
        .query(&[SqlParameter::Offset(SqlValue::Int(3))])
        .await
        .unwrap();

    let row = query.next().await.unwrap().unwrap();

    assert_eq!(row.ensure_int(0).unwrap(), 3);

    let query = prepare
        .query(&[SqlParameter::Offset(SqlValue::Int(4))])
        .await
        .unwrap();

    let row = query.next().await.unwrap().unwrap();

    assert_eq!(row.ensure_int(0).unwrap(), 2);

    let query = prepare
        .query(&[SqlParameter::Offset(SqlValue::Int(5))])
        .await
        .unwrap();

    let row = query.next().await.unwrap().unwrap();

    assert_eq!(row.ensure_int(0).unwrap(), 1);

    let query = prepare
        .query(&[SqlParameter::Offset(SqlValue::Int(6))])
        .await
        .unwrap();

    let row = query.next().await.unwrap().unwrap();

    assert_eq!(row.ensure_int(0).unwrap(), 0);
}
