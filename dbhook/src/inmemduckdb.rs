use crate::genericdb::GenericDB;
use anyhow::bail;
use duckdb::Connection;
use instruments::instrument::Instrument;

struct ParticipantPassword {
    participant: u64,
    password: String,
}

/// In memory DuckDB, used mostly for hacking and quick dirty tests
/// ::connect's DBName needs to be the type of the database.
/// E.g.: "csv", "parquet" or other
///
/// The files should be called users.type and instruments.type
/// E.g. "users.csv" and "instruments.csv"
///
/// Example:
///
/// ```
/// use InMemDuckDB;
///
/// let db = InMemDuckDB::default();
/// db.connect("", 0, "", "", "csv");
/// let instruments = db.get_instruments();
/// db.disconnect()
/// ```
///
pub struct InMemDuckDB {
    connection: Connection,
    dbname: String,
}

impl Default for InMemDuckDB {
    fn default() -> Self {
        Self {
            connection: Connection::open_in_memory().unwrap(),
            dbname: String::from("connect_me_first"),
        }
    }
}

impl GenericDB for InMemDuckDB {
    fn connect(
        &mut self,
        _addr: &str,
        _port: u16,
        _username: &str,
        _password: &str,
        dbname: &str,
    ) -> anyhow::Result<()> {
        self.dbname = String::from(dbname);
        Ok(())
    }

    fn disconnect(&mut self) {}

    fn check_login(
        &mut self,
        username: &str,
        password_hash: &[u8; 64],
        session_id: u32,
    ) -> anyhow::Result<u64> {
        let mut prepared_statement = self.connection.prepare(
            "SELECT participant, password from 'users.?' WHERE username=? and session_id=?",
        )?;
        let matches = prepared_statement.query_map(
            [&self.dbname, username, &format!("{session_id}")],
            |row| {
                Ok(ParticipantPassword {
                    participant: row.get(0)?,
                    password: row.get(1)?,
                })
            },
        )?;
        for m in matches {
            let pp = m?;
            let hashed_password = oep::login::Login::free_text_hash(&pp.password);
            if password_hash.eq(&hashed_password) == true {
                return Ok(pp.participant);
            }
            break;
        }
        bail!("Invalid password");
    }

    fn get_instruments(&mut self) -> Vec<instruments::instrument::Instrument> {
        let mut prepared_statement = self
            .connection
            .prepare(
                "SELECT id, name, i_type, state, percentage_bands, percentage_variation_allowed
            from 'instruments.?' where active = 1",
            )
            .unwrap();
        let matches = prepared_statement
            .query_map([&self.dbname], |row| {
                Ok(Instrument::new(
                    row.get(0).unwrap(),
                    &row.get::<_, String>(1).unwrap(),
                    row.get::<_, u8>(2).unwrap().into(),
                    row.get::<_, u8>(3).unwrap().into(),
                    row.get(4).unwrap(),
                    row.get(5).unwrap(),
                ))
            })
            .unwrap();
        matches.map(|res| res.unwrap()).collect()
    }
}
