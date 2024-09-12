use crate::genericdb::GenericDB;
use anyhow::{anyhow, bail, Result};
use instruments::instrument::Instrument;
use postgres::Client;

pub struct PGSqlDB {
    client: Option<Client>,
}

impl PGSqlDB {}

impl Default for PGSqlDB {
    fn default() -> Self {
        Self { client: None }
    }
}

impl GenericDB for PGSqlDB {
    fn connect(
        &mut self,
        addr: &str,
        port: u16,
        username: &str,
        password: &str,
        dbname: &str,
    ) -> Result<()> {
        self.client = Some(Client::connect(
            format!("host={addr} port={port} user={username} password={password} dbname={dbname}")
                .as_str(),
            postgres::NoTls,
        )?);
        Ok(())
    }

    fn disconnect(&mut self) {
        let mut t = None;
        std::mem::swap(&mut t, &mut self.client);

        if let Some(c) = t {
            c.close().unwrap_or_default();
        }
    }

    fn check_login(
        &mut self,
        username: &str,
        password_hash: &[u8; 64],
        session_id: u32,
    ) -> anyhow::Result<u64> {
        let s_id = session_id as i32;
        let query = self.client.as_mut().unwrap().query(
            "SELECT participant, password from users where
            username=$1 AND session_id=$2",
            &[&username, &s_id],
        )?;
        if query.len() == 0 {
            return Err(anyhow!(format!(
                "Invalid credentials for {username}/{s_id}"
            )));
        } else if query.len() > 1 {
            return Err(anyhow!(format!("Too many matches for {username}")));
        }
        let password: String = query[0].get("password");
        let hashed_password = oep::login::Login::free_text_hash(&password);
        if password_hash.eq(&hashed_password) == false {
            bail!("Invalid password");
        }
        let participant: i64 = query[0].get("participant");
        return Ok(participant as u64);
    }

    fn get_instruments(&mut self) -> Vec<Instrument> {
        let query = self.client.as_mut().unwrap().query(
            "SELECT id, name, i_type, state, percentage_bands, percentage_variation_allowed
            from instrument where active = 1",
            &[],
        );
        match query {
            Ok(result) => result
                .iter()
                .map(|x| {
                    let id: i64 = x.get(0);
                    let name: String = x.get(1);
                    let i_type: i16 = x.get(2);
                    let state: i16 = x.get(3);
                    let perc_bands: i16 = x.get(4);
                    let perc_var_allowed: i16 = x.get(5);
                    Instrument::new(
                        id as u64,
                        &name,
                        (i_type as u8).into(),
                        (state as u8).into(),
                        perc_bands as u8,
                        perc_var_allowed as u8,
                    )
                })
                .collect(),
            Err(_) => vec![],
        }
    }
}
