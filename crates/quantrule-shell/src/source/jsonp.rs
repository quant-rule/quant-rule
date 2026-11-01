//! The Dukascopy JSONP payload parser.
//!
//! The endpoint wraps a JSON array of rows in a callback:
//! `callback([[timestamp, open, high, low, close, volume], ...]);`.
//! The callback name is not significant. The parser accepts any prefix before
//! the first `(` and any suffix after the last `)`, which also tolerates the
//! trailing semicolon the provider sends.
//!
//! Volume is optional in this parser. A JSON `null` or a missing sixth field
//! becomes `None`; a present number stays `Some`, including a real zero.

use std::fmt;

use serde_json::Value;

/// One numeric row from a provider page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PageRow {
    /// The candle timestamp in UTC Unix milliseconds.
    pub(crate) milliseconds: i64,
    /// Open price.
    pub(crate) open: f64,
    /// High price.
    pub(crate) high: f64,
    /// Low price.
    pub(crate) low: f64,
    /// Close price.
    pub(crate) close: f64,
    /// Volume, when the provider supplied one.
    pub(crate) volume: Option<f64>,
}

/// A provider payload could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum JsonpError {
    /// No callback parentheses were found.
    MissingCallback,
    /// The callback parentheses are in the wrong order.
    MalformedCallback,
    /// The JSON inside the callback is not valid.
    InvalidJson(String),
    /// The JSON value is not an array.
    NotAnArray,
    /// One row is not a six-element array.
    RowShape {
        /// The zero-based row index.
        index: usize,
        /// What the parser expected.
        expected: &'static str,
    },
    /// One row has a non-numeric or out-of-range field.
    RowField {
        /// The zero-based row index.
        index: usize,
        /// The field name.
        field: &'static str,
    },
    /// One row has a non-finite number.
    NonFinite {
        /// The zero-based row index.
        index: usize,
        /// The field name.
        field: &'static str,
    },
}

impl fmt::Display for JsonpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCallback => formatter.write_str("response has no JSONP callback"),
            Self::MalformedCallback => formatter.write_str("response callback is malformed"),
            Self::InvalidJson(message) => write!(formatter, "response JSON is invalid: {message}"),
            Self::NotAnArray => formatter.write_str("response JSON is not an array"),
            Self::RowShape { index, expected } => {
                write!(formatter, "row {index} is not {expected}")
            }
            Self::RowField { index, field } => {
                write!(formatter, "row {index} has an invalid {field}")
            }
            Self::NonFinite { index, field } => {
                write!(formatter, "row {index} has a non-finite {field}")
            }
        }
    }
}

impl std::error::Error for JsonpError {}

/// Parses a JSONP page into numeric rows.
///
/// Empty pages are valid and produce an empty vector. A page is parsed
/// completely before any row is returned, so a malformed row cannot leave a
/// partially accepted page.
///
/// # Errors
///
/// Returns a [`JsonpError`] when the callback wrapper is missing or malformed,
/// the enclosed JSON is invalid or not an array, or a row does not have the
/// expected six numeric fields.
pub(crate) fn parse_jsonp(body: &str) -> Result<Vec<PageRow>, JsonpError> {
    let open = body.find('(').ok_or(JsonpError::MissingCallback)?;
    let close = body.rfind(')').ok_or(JsonpError::MissingCallback)?;
    if close <= open {
        return Err(JsonpError::MalformedCallback);
    }

    let value: Value = serde_json::from_str(&body[open + 1..close])
        .map_err(|error| JsonpError::InvalidJson(error.to_string()))?;
    let rows = value.as_array().ok_or(JsonpError::NotAnArray)?;
    let mut parsed = Vec::with_capacity(rows.len());

    for (index, row) in rows.iter().enumerate() {
        let fields = row.as_array().ok_or(JsonpError::RowShape {
            index,
            expected: "an array",
        })?;
        if fields.len() < 5 {
            return Err(JsonpError::RowShape {
                index,
                expected: "an array of at least five fields",
            });
        }

        let milliseconds = fields[0].as_i64().ok_or(JsonpError::RowField {
            index,
            field: "timestamp",
        })?;
        if !(0..=quantrule_data::MAX_TIMESTAMP_MILLISECONDS).contains(&milliseconds) {
            return Err(JsonpError::RowField {
                index,
                field: "timestamp",
            });
        }

        let open = price_field(fields, index, 1, "open")?;
        let high = price_field(fields, index, 2, "high")?;
        let low = price_field(fields, index, 3, "low")?;
        let close = price_field(fields, index, 4, "close")?;
        let volume = match fields.get(5) {
            None | Some(Value::Null) => None,
            Some(value) => {
                let volume = value.as_f64().ok_or(JsonpError::RowField {
                    index,
                    field: "volume",
                })?;
                if !volume.is_finite() {
                    return Err(JsonpError::NonFinite {
                        index,
                        field: "volume",
                    });
                }
                Some(volume)
            }
        };

        parsed.push(PageRow {
            milliseconds,
            open,
            high,
            low,
            close,
            volume,
        });
    }

    Ok(parsed)
}

fn price_field(
    fields: &[Value],
    index: usize,
    position: usize,
    field: &'static str,
) -> Result<f64, JsonpError> {
    let value = fields[position]
        .as_f64()
        .ok_or(JsonpError::RowField { index, field })?;
    if !value.is_finite() {
        return Err(JsonpError::NonFinite { index, field });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{JsonpError, PageRow, parse_jsonp};

    fn page(rows: &str) -> String {
        format!("qr({rows});")
    }

    #[test]
    fn parses_a_page_with_a_null_volume() {
        let body = page("[[0,1.0,2.0,0.5,1.5,10.0],[60000,1.5,2.5,1.0,2.0,null]]");

        let rows = parse_jsonp(&body).expect("the page is valid");

        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0],
            PageRow {
                milliseconds: 0,
                open: 1.0,
                high: 2.0,
                low: 0.5,
                close: 1.5,
                volume: Some(10.0),
            }
        );
        assert_eq!(rows[1].volume, None);
    }

    #[test]
    fn accepts_a_missing_volume_field() {
        let rows = parse_jsonp(&page("[[0,1,2,0.5,1.5]]")).expect("volume is optional");

        assert_eq!(rows[0].volume, None);
    }

    #[test]
    fn accepts_an_empty_page() {
        assert_eq!(parse_jsonp(&page("[]")), Ok(Vec::new()));
    }

    #[test]
    fn accepts_whitespace_and_a_callback_with_any_name() {
        let rows = parse_jsonp(" callback_name( [ [0, 1, 2, 0.5, 1.5, 0] ] );\n")
            .expect("whitespace is harmless");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].milliseconds, 0);
    }

    #[test]
    fn rejects_a_missing_callback() {
        assert_eq!(
            parse_jsonp("[[0,1,2,0.5,1.5,0]]"),
            Err(JsonpError::MissingCallback)
        );
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(matches!(
            parse_jsonp("qr([[0,1,2]);"),
            Err(JsonpError::InvalidJson(_))
        ));
    }

    #[test]
    fn rejects_a_non_array_json_value() {
        assert_eq!(parse_jsonp("qr({})"), Err(JsonpError::NotAnArray));
    }

    #[test]
    fn rejects_a_short_row() {
        assert!(matches!(
            parse_jsonp("qr([[0,1,2,3]])"),
            Err(JsonpError::RowShape { index: 0, .. })
        ));
    }

    #[test]
    fn rejects_a_non_numeric_price() {
        assert!(matches!(
            parse_jsonp("qr([[0,1,\"two\",0.5,1.5,0]])"),
            Err(JsonpError::RowField {
                index: 0,
                field: "high"
            })
        ));
    }

    #[test]
    fn rejects_a_negative_timestamp() {
        assert!(matches!(
            parse_jsonp("qr([[-1,1,2,0.5,1.5,0]])"),
            Err(JsonpError::RowField {
                index: 0,
                field: "timestamp"
            })
        ));
    }

    #[test]
    fn rejects_a_timestamp_above_the_supported_range() {
        let body = format!(
            "qr([[{},1,2,0.5,1.5,0]])",
            quantrule_data::MAX_TIMESTAMP_MILLISECONDS + 1
        );
        assert!(matches!(
            parse_jsonp(&body),
            Err(JsonpError::RowField {
                index: 0,
                field: "timestamp"
            })
        ));
    }
}
