#[cfg(test)]
mod tests {

    use reweb3_num::{
        cast::As,
        types::{I256, U256},
    };
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Serialize, Deserialize)]
    struct Mock {
        value1: I256,
        value2: U256,
        value3: I256,
    }

    #[test]
    fn test_num() {
        let value = json!({
            "value1": -1,
            "value2": "0x1",
            "value3":"1"
        });

        let mock = serde_json::from_value::<Mock>(value).unwrap();

        assert_eq!(mock.value1, I256::from(-1));

        assert_eq!(mock.value2, I256::from(1).as_());

        assert_eq!(mock.value3, I256::from(1));
    }
}
