use anyhow::Result;
use instruments::instrument::Instrument;

pub trait GenericDB {
    fn connect(
        &mut self,
        addr: &str,
        port: u16,
        username: &str,
        password: &str,
        dbname: &str,
    ) -> Result<()>;
    fn disconnect(&mut self);
    fn check_login(&mut self, username: &str, password: &[u8; 64], session_id: u32) -> Result<u64>;
    fn get_instruments(&mut self) -> Vec<Instrument>;
}
