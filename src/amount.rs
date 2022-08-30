use serde::{Deserialize, Serialize};

const NUM_DIGITS: usize = 4;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Amount(pub i64);

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = self.0.to_string();
        if s.len() <= NUM_DIGITS {
            let pad = NUM_DIGITS + 1 - s.len();
            s.insert_str(0, "0".repeat(pad).as_str());
        }
        s.insert(s.len() - NUM_DIGITS, '.');
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for Amount {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = s.to_string();

        let pad_digits = if let Some(dec_pos) = s.rfind('.') {
            s.replace_range(dec_pos..dec_pos + 1, "");
            s.len() - dec_pos
        } else {
            0
        };
        if pad_digits > NUM_DIGITS {
            // trim
            s.replace_range(s.len() - (pad_digits - NUM_DIGITS)..s.len(), "")
        } else {
            // pad
            let pad = "0".repeat(NUM_DIGITS - pad_digits);
            s.push_str(pad.as_str());
        }
        s.parse::<i64>()
            .map_err(|_| "invalid amount")
            // .and_then(|res| {
            //     if res < 0 {
            //         Err("amount must be positive")
            //     } else {
            //         Ok(res)
            //     }
            // })
            .map(Amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_parse(s: &str, a: i64) {
        assert_eq!(s.parse::<Amount>().unwrap(), Amount(a));
    }

    fn assert_string(a: i64, s: &str) {
        assert_eq!(Amount(a).to_string(), s)
    }

    #[test]
    fn amount_parse() {
        assert_parse("1", 10000);
        assert_parse("1.0", 10000);
        assert_parse("1.1234", 11234);
        assert_parse("1.12345", 11234);
        assert_eq!(
            "1n".parse::<Amount>().unwrap_err(),
            "invalid amount".to_owned()
        );
        // assert_eq!(
        //     "-1".parse::<Amount>().unwrap_err(),
        //     "amount must be positive".to_owned()
        // );
    }

    #[test]
    fn amount_to_string() {
        assert_string(0, "0.0000");
        assert_string(10, "0.0010");
        assert_string(10000, "1.0000");
    }
}
