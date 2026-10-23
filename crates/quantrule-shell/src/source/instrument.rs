//! Instrument names and their provider symbols.
//!
//! The provider identifies an instrument with a mixed-case symbol such as
//! `XAU/USD` or `E_SandP-500`, while the file name needs a compact slug such as
//! `XAUUSD`. The shell owns this mapping because it is source-specific; the
//! data engine never sees an instrument name.

use std::fmt;

/// User-facing aliases and their provider symbols.
const ALIASES: &[(&str, &str)] = &[
    ("GOLD", "XAU/USD"),
    ("GOLD/USD", "XAU/USD"),
    ("XAUUSD", "XAU/USD"),
    ("XAU/USD", "XAU/USD"),
    ("SILVER", "XAG/USD"),
    ("SILVER/USD", "XAG/USD"),
    ("XAGUSD", "XAG/USD"),
    ("XAG/USD", "XAG/USD"),
    ("EURUSD", "EUR/USD"),
    ("EUR/USD", "EUR/USD"),
    ("GBPUSD", "GBP/USD"),
    ("GBP/USD", "GBP/USD"),
    ("USDJPY", "USD/JPY"),
    ("USD/JPY", "USD/JPY"),
    ("BTCUSD", "BTC/USD"),
    ("BTC/USD", "BTC/USD"),
    ("ETHUSD", "ETH/USD"),
    ("ETH/USD", "ETH/USD"),
    ("SP500", "E_SandP-500"),
    ("SPX", "E_SandP-500"),
    ("US500", "E_SandP-500"),
    ("NAS100", "E_NQ-100"),
    ("NASDAQ100", "E_NQ-100"),
    ("US100", "E_NQ-100"),
    ("TSLA", "TSLA.US/USD"),
    ("TESLA", "TSLA.US/USD"),
    ("AAPL", "AAPL.US/USD"),
    ("APPLE", "AAPL.US/USD"),
    ("NVDA", "NVDA.US/USD"),
    ("GOOGL", "GOOGL.US/USD"),
    ("WTI", "E_Light"),
    ("OIL", "E_Light"),
    ("CRUDE", "E_Light"),
    ("BRENT", "E_Brent"),
];

/// Provider symbols whose canonical spelling is known.
const PROVIDER_SYMBOLS: &[&str] = &[
    "XAU/USD",
    "XAG/USD",
    "EUR/USD",
    "GBP/USD",
    "USD/JPY",
    "BTC/USD",
    "ETH/USD",
    "E_SandP-500",
    "E_NQ-100",
    "TSLA.US/USD",
    "AAPL.US/USD",
    "NVDA.US/USD",
    "GOOGL.US/USD",
    "E_Light",
    "E_Brent",
];

/// A resolved provider instrument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Symbol {
    provider: String,
    slug: String,
}

impl Symbol {
    /// Resolves a user-facing alias or a provider symbol.
    ///
    /// Matching is case-insensitive for the known aliases and provider symbols.
    /// An unknown value is accepted only when it looks like a provider symbol,
    /// which means it contains `/`, `_`, `-`, or `.`; the provider remains the
    /// final authority on whether that instrument exists.
    ///
    /// # Errors
    ///
    /// Returns [`SymbolError::Empty`] for blank input and
    /// [`SymbolError::Unknown`] for a bare name that is neither a known alias
    /// nor provider-shaped.
    pub(crate) fn resolve(input: &str) -> Result<Self, SymbolError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(SymbolError::Empty);
        }

        let provider = if let Some(provider) = resolve_alias(input) {
            provider.to_owned()
        } else if let Some(provider) = resolve_provider_symbol(input) {
            provider.to_owned()
        } else if is_provider_shape(input) {
            input.to_owned()
        } else {
            return Err(SymbolError::Unknown(input.to_owned()));
        };

        let slug = slug_from_provider(&provider);
        if slug.is_empty() {
            return Err(SymbolError::Unknown(input.to_owned()));
        }

        Ok(Self { provider, slug })
    }

    /// The provider's symbol, such as `XAU/USD`.
    pub(crate) fn provider(&self) -> &str {
        &self.provider
    }

    /// The compact uppercase slug used in file names, such as `XAUUSD`.
    pub(crate) fn slug(&self) -> &str {
        &self.slug
    }
}

/// A symbol could not be resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SymbolError {
    /// The input was empty or whitespace only.
    Empty,
    /// The input is not a known alias and not provider-shaped.
    Unknown(String),
}

impl fmt::Display for SymbolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("symbol is empty"),
            Self::Unknown(symbol) => write!(
                formatter,
                "unknown symbol '{symbol}'; try gold, silver, eurusd, gbpusd, btcusd, sp500, or a provider symbol such as XAU/USD"
            ),
        }
    }
}

impl std::error::Error for SymbolError {}

fn is_provider_shape(input: &str) -> bool {
    input
        .chars()
        .any(|character| matches!(character, '/' | '_' | '-' | '.'))
        && input.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '/' | '_' | '-' | '.')
        })
}

fn resolve_alias(input: &str) -> Option<&'static str> {
    let upper = input.to_ascii_uppercase();
    ALIASES
        .iter()
        .find(|(alias, _)| *alias == upper.as_str())
        .map(|(_, provider)| *provider)
}

fn resolve_provider_symbol(input: &str) -> Option<&'static str> {
    PROVIDER_SYMBOLS
        .iter()
        .find(|provider| provider.eq_ignore_ascii_case(input))
        .copied()
}

fn slug_from_provider(provider: &str) -> String {
    provider
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .map(|character| character.to_ascii_uppercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Symbol, SymbolError};

    #[test]
    fn resolves_the_default_gold_alias() {
        let symbol = Symbol::resolve("gold").expect("gold is a known alias");

        assert_eq!(symbol.provider(), "XAU/USD");
        assert_eq!(symbol.slug(), "XAUUSD");
    }

    #[test]
    fn resolves_a_provider_symbol_case_insensitively() {
        let symbol = Symbol::resolve("xau/usd").expect("the symbol is known");

        assert_eq!(symbol.provider(), "XAU/USD");
        assert_eq!(symbol.slug(), "XAUUSD");
    }

    #[test]
    fn resolves_a_mixed_case_provider_symbol() {
        let symbol = Symbol::resolve("e_sandp-500").expect("the symbol is known");

        assert_eq!(symbol.provider(), "E_SandP-500");
        assert_eq!(symbol.slug(), "ESANDP500");
    }

    #[test]
    fn accepts_an_unknown_provider_shaped_symbol() {
        let symbol = Symbol::resolve("ABC.US/USD").expect("the provider shape is accepted");

        assert_eq!(symbol.provider(), "ABC.US/USD");
        assert_eq!(symbol.slug(), "ABCUSUSD");
    }

    #[test]
    fn rejects_a_bare_unknown_name() {
        assert_eq!(
            Symbol::resolve("notreal"),
            Err(SymbolError::Unknown(String::from("notreal")))
        );
    }

    #[test]
    fn rejects_a_provider_symbol_with_disallowed_characters() {
        assert_eq!(
            Symbol::resolve("XAU /USD"),
            Err(SymbolError::Unknown(String::from("XAU /USD")))
        );
        assert_eq!(
            Symbol::resolve("XAU/US\nD"),
            Err(SymbolError::Unknown(String::from("XAU/US\nD")))
        );
    }

    #[test]
    fn rejects_a_separator_only_symbol() {
        assert_eq!(
            Symbol::resolve("///"),
            Err(SymbolError::Unknown(String::from("///")))
        );
    }

    #[test]
    fn rejects_blank_input() {
        assert_eq!(Symbol::resolve("  "), Err(SymbolError::Empty));
    }
}
