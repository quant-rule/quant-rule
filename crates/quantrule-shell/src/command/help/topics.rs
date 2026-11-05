//! Static reference topics for the shell help.
//!
//! The glossary is the one topic that is not generated from the command table.
//! It defines the precise terms the command output uses.

use std::io::{self, Write};

/// Renders the precise-term glossary.
pub(super) fn render_glossary(writer: &mut impl Write) -> io::Result<()> {
    writeln!(writer, "Glossary")?;
    writeln!(writer)?;
    writeln!(
        writer,
        "candle        One closed OHLCV bar. The model consumes candles only."
    )?;
    writeln!(
        writer,
        "timeframe     A first-class candle series. The default ladder is 1m, 5m, 15m, 1h."
    )?;
    writeln!(
        writer,
        "reading       A published indicator value, such as EMA, RSI, ATR, RVOL, MACD, or ADX/DI."
    )?;
    writeln!(
        writer,
        "state         A discrete code derived from a reading, independent of any axis score."
    )?;
    writeln!(
        writer,
        "level         A confirmed swing pivot, with occurrence and availability bars."
    )?;
    writeln!(
        writer,
        "event         A Level, Divergence, or Signal record; it is evidence, not a vote."
    )?;
    writeln!(
        writer,
        "progression   The structural leg at the minor and major scales, with its lifecycle and stage."
    )?;
    writeln!(
        writer,
        "context       Volatility and participation measurements, such as ATR ratio and RVOL ratio."
    )?;
    writeln!(
        writer,
        "availability  The first closed bar on which a measurement became knowable."
    )?;
    writeln!(
        writer,
        "M1.0          The current flat per-timeframe model: readings, states, levels, events, progression, and context."
    )?;
    writeln!(
        writer,
        "legacy axis   The retired Direction, Momentum, Structure, and Progression scores. No current command publishes them."
    )
}
