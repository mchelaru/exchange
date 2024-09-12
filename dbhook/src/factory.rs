use crate::genericdb::GenericDB;
#[cfg(feature = "duckdb")]
use crate::inmemduckdb::InMemDuckDB;
use crate::mockdb::MockDB;
#[cfg(feature = "postgres")]
use crate::pgsqldb::PGSqlDB;

pub fn build(dbtype: &str) -> Box<dyn GenericDB> {
    match dbtype {
        #[cfg(feature = "duckdb")]
        "inmemduckdb" => Box::new(InMemDuckDB::default()),
        #[cfg(feature = "postgres")]
        "pgsql" => Box::new(PGSqlDB::default()),
        "mock" => Box::new(MockDB::default()),
        _ => panic!("No such DB type: {dbtype}"),
    }
}

#[cfg(test)]
mod test {
    use super::build;

    #[test]
    fn builds_all_variants() {
        let _v = [
            #[cfg(feature = "postgres")]
            "pgsql",
            "mock",
            #[cfg(feature = "duckdb")]
            "inmemduckdb",
        ]
        .map(|x| build(x));
    }

    #[test]
    #[should_panic]
    fn build_panics_on_unknown() {
        let _v = ["something"].map(|x| build(x));
    }

    #[test]
    fn no_connect_disconnect_doesnt_panic() {
        let v = [
            #[cfg(feature = "postgres")]
            "pgsql",
            "mock",
            #[cfg(feature = "duckdb")]
            "inmemduckdb",
        ]
        .map(|x| build(x));
        v.map(|mut i| i.disconnect());
    }
}
