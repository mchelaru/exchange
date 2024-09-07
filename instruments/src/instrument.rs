use std::hash::Hash;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum InstrumentState {
    Trading,
    Closed,
    Auction,
}

impl Into<u8> for InstrumentState {
    fn into(self) -> u8 {
        match self {
            Self::Trading => 0,
            Self::Closed => 1,
            Self::Auction => 2,
        }
    }
}

impl From<u8> for InstrumentState {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Trading,
            1 => Self::Closed,
            2 => Self::Auction,
            _ => Self::Closed,
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum InstrumentType {
    Share,
    OptionCall,
    OptionPut,
    Future,
    Warrant,
}

impl Into<u8> for InstrumentType {
    fn into(self) -> u8 {
        match self {
            Self::Share => 0,
            Self::OptionCall => 1,
            Self::OptionPut => 2,
            Self::Future => 3,
            Self::Warrant => 4,
        }
    }
}

impl From<u8> for InstrumentType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Share,
            1 => Self::OptionCall,
            2 => Self::OptionPut,
            3 => Self::Future,
            4 => Self::Warrant,
            _ => Self::Share,
        }
    }
}

#[derive(Eq, Debug, Clone)]
pub struct Instrument {
    id: u64,
    name: String,
    // type of the instrument: stock, option etc.
    i_type: InstrumentType,
    // state: trading, auction, closed etc.
    state: InstrumentState,
    // fractional as hundreds of percentage
    percentage_bands: u8,
    // percentage of allowed daily variation
    percentage_variation_allowed: u8,
}

impl Instrument {
    pub fn new(
        id: u64,
        name: &str,
        i_type: InstrumentType,
        state: InstrumentState,
        percentage_bands: u8,
        percentage_variation_allowed: u8,
    ) -> Self {
        Self {
            id: id,
            name: String::from(name),
            i_type: i_type,
            state: state,
            percentage_bands: percentage_bands,
            percentage_variation_allowed: percentage_variation_allowed,
        }
    }

    /// used mostly in tests
    pub fn new_fast(id: u64, i_type: InstrumentType) -> Self {
        Self {
            id: id,
            name: String::from(""),
            i_type: i_type,
            state: InstrumentState::Closed,
            percentage_bands: 0,
            percentage_variation_allowed: 30,
        }
    }

    pub fn copy(i: &Instrument) -> Self {
        Self {
            id: i.id,
            name: i.name.clone(),
            i_type: i.i_type,
            state: i.state,
            percentage_bands: i.percentage_bands,
            percentage_variation_allowed: i.percentage_variation_allowed,
        }
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_type(&self) -> InstrumentType {
        self.i_type
    }

    pub fn get_state(&self) -> InstrumentState {
        self.state
    }

    pub fn set_state(&mut self, state: InstrumentState) {
        self.state = state;
    }

    pub fn set_percentage_bands(&mut self, percentage_bands: u8) {
        assert!(percentage_bands <= 100);
        self.percentage_bands = percentage_bands;
    }

    pub fn get_percentage_bands(&self) -> u8 {
        self.percentage_bands
    }

    pub fn set_percentage_variation_allowed(&mut self, percentage_variation_allowed: u8) {
        self.percentage_variation_allowed = percentage_variation_allowed;
    }

    pub fn get_percentage_variation_allowed(&self) -> u8 {
        self.percentage_variation_allowed
    }

    /// encode the instrument e.g. in order to send it over feed
    pub fn encode(&self) -> Vec<u8> {
        let mut r = vec![];
        r.extend_from_slice(&self.get_id().to_le_bytes());
        r.extend_from_slice(&[
            self.get_type().into(),
            self.get_state().into(),
            self.get_percentage_bands(),
            self.get_percentage_variation_allowed(),
        ]);
        r.extend_from_slice(self.get_name().as_bytes());
        r
    }

    // FIXME: since we have a String, this will never work
    pub fn decode(buf: &[u8]) -> Self {
        Self {
            id: u64::from_le_bytes(buf[0..8].try_into().unwrap()),
            name: String::from_utf8(buf[12..].to_vec()).unwrap(),
            i_type: buf[8].into(),
            state: buf[9].into(),
            percentage_bands: buf[10].into(),
            percentage_variation_allowed: buf[11].into(),
        }
    }
}

impl PartialEq for Instrument {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for Instrument {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.id);
    }
}

#[cfg(test)]
mod test {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    //    use crate::instruments::instrument::InstrumentType;

    use crate::instrument::InstrumentType;

    use super::Instrument;

    fn calculate_hash(instrument: &Instrument) -> u64 {
        let mut hasher = DefaultHasher::new();
        instrument.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn same_hash() {
        let i1 = Instrument::new_fast(100, InstrumentType::Share);
        let i2 = Instrument::new_fast(100, InstrumentType::Share);

        assert_eq!(calculate_hash(&i1), calculate_hash(&i2));
    }

    #[test]
    fn diff_instrument_type_same_hash() {
        let i1 = Instrument::new_fast(100, InstrumentType::Share);
        let i2 = Instrument::new_fast(100, InstrumentType::OptionCall);

        assert_eq!(calculate_hash(&i1), calculate_hash(&i2));
    }

    #[test]
    fn diff_hash() {
        let i1 = Instrument::new_fast(100, InstrumentType::Share);
        let i2 = Instrument::new_fast(200, InstrumentType::Share);

        assert_ne!(calculate_hash(&i1), calculate_hash(&i2));
    }
}
