use serde::{Deserialize, Deserializer, Serializer};

const NUM_DIGITS: usize = 4;

pub fn serialize<S>(amount: &i64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut str = amount.to_string();
    if str.len() <= NUM_DIGITS {
        let pad = NUM_DIGITS + 1 - str.len();
        str.insert_str(0, "0".repeat(pad).as_str());
    }
    str.insert(str.len() - NUM_DIGITS, '.');
    s.serialize_str(str.as_str())
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<&str> = Deserialize::deserialize(deserializer)?;

    if s.is_none() {
        return Ok(None);
    }

    let mut s = s.unwrap().to_string();
    let pad_digits = if let Some(dec_pos) = s.rfind('.') {
        // remove '.'
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
    s.parse::<i64>().map(Some).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde::Serialize;
    use serde_test::{assert_de_tokens, assert_de_tokens_error, assert_ser_tokens, Token};

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct In {
        #[serde(with = "super")]
        value: Option<i64>,
    }

    #[derive(Debug, Serialize, PartialEq, Eq)]
    struct Out {
        #[serde(with = "super")]
        value: i64,
    }

    fn assert_de(s: &'static str, value: Option<i64>) {
        if value.is_some() {
            assert_de_tokens(
                &In { value },
                &[
                    Token::Struct { name: "In", len: 1 },
                    Token::Str("value"),
                    Token::Some,
                    Token::BorrowedStr(s),
                    Token::StructEnd,
                ],
            );
        } else {
            assert_de_tokens(
                &In { value },
                &[
                    Token::Struct { name: "In", len: 1 },
                    Token::Str("value"),
                    Token::None,
                    Token::StructEnd,
                ],
            );
        };
    }

    #[test]
    fn de() {
        assert_de("1", Some(10000));
        assert_de("1.0", Some(10000));
        assert_de("1.1234", Some(11234));
        assert_de("1.12345", Some(11234));
        assert_de("-1.12345", Some(-11234));
        assert_de("", None);

        assert_de_tokens_error::<In>(
            &[
                Token::Struct { name: "In", len: 1 },
                Token::Str("value"),
                Token::Some,
                Token::BorrowedStr("x"),
                Token::StructEnd,
            ],
            "invalid digit found in string",
        );
    }

    fn assert_ser(value: i64, s: &'static str) {
        assert_ser_tokens(
            &Out { value },
            &[
                Token::Struct {
                    name: "Out",
                    len: 1,
                },
                Token::Str("value"),
                Token::BorrowedStr(s),
                Token::StructEnd,
            ],
        )
    }

    #[test]
    fn ser() {
        assert_ser(0, "0.0000");
        assert_ser(10, "0.0010");
        assert_ser(10000, "1.0000");
        assert_ser(-10000, "-1.0000");
    }
}
