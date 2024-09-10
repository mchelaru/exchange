use std::collections::HashMap;

pub fn get_config_string(
    config_map: &HashMap<String, HashMap<String, Option<String>>>,
    section: &str,
    key: &str,
) -> String {
    config_map
        .get(section)
        .expect(format!("No {section} section found").as_str())
        .get(key)
        .expect(
            format!("Please set the {key} in the {section} section of your config file.").as_str(),
        )
        .as_ref()
        .expect("")
        .clone()
}

#[cfg(test)]
mod test {
    use configparser::ini::Ini;

    use crate::config::get_config_string;

    #[test]
    fn load_config() {
        let mut f = Ini::new();
        let config_map = f
            .read(String::from(
                "[database]
        addr=192.168.1.1
        port=8192
        username=test
        
        [gateway]
        id=55",
            ))
            .unwrap();

        assert_eq!("8192", get_config_string(&config_map, "database", "port"));
        assert_eq!(
            "192.168.1.1",
            get_config_string(&config_map, "database", "addr")
        );
        assert_eq!(
            "test",
            get_config_string(&config_map, "database", "username")
        );
        assert_eq!("55", get_config_string(&config_map, "gateway", "id"));
    }

    #[test]
    #[should_panic]
    fn get_missing_section() {
        let mut f = Ini::new();
        let config_map = f
            .read(String::from(
                "[database]
        addr=192.168.1.1
        port=8192
        username=test
        
        [gateway]
        id=55",
            ))
            .unwrap();

        let _value = get_config_string(&config_map, "notakey", "port");
    }

    #[test]
    #[should_panic]
    fn get_missing_key() {
        let mut f = Ini::new();
        let config_map = f
            .read(String::from(
                "[database]
        addr=192.168.1.1
        port=8192
        username=test
        
        [gateway]
        id=55",
            ))
            .unwrap();

        let _value = get_config_string(&config_map, "database", "password");
    }
}
