use crate::genericdb::GenericDB;

pub struct MockDB {}

impl Default for MockDB {
    fn default() -> Self {
        Self {}
    }
}

impl GenericDB for MockDB {
    fn connect(
        &mut self,
        _addr: &str,
        _port: u16,
        _username: &str,
        _password: &str,
        _dbname: &str,
    ) -> anyhow::Result<()> {
        todo!()
    }

    fn check_login(
        &mut self,
        _username: &str,
        _password: &[u8; 64],
        _session_id: u32,
    ) -> anyhow::Result<u64> {
        Ok(111)
    }

    fn disconnect(&mut self) {}

    fn get_instruments(&mut self) -> Vec<instruments::instrument::Instrument> {
        todo!()
    }
}
