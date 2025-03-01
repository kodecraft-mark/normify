use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use tracing::error;

/// Module containing exchange-related definitions
pub mod exchange;

/// Standard date format for expiry parsing
const STANDARD_DATE_FORMAT: &str = "%Y%m%d";

/// Represents different exchanges
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Exchange {
    Deribit,
    Dydx,
    Derive,
    Paradex
}

impl ToString for Exchange {
    /// Converts an `Exchange` to its string representation
    fn to_string(&self) -> String {
        match self {
            Exchange::Deribit => "deribit".to_string(),
            Exchange::Dydx => "dydx".to_string(),
            Exchange::Derive => "derive".to_string(),
            Exchange::Paradex => "paradex".to_string()
        }
    }
}

/// Represents different market types
#[derive(Debug, PartialEq)]
pub enum MarketType {
    OrderBook,
    PublicTrades,
    Ticker
}

impl ToString for MarketType {
    /// Converts a `MarketType` to its string representation
    fn to_string(&self) -> String {
        match self {
            MarketType::OrderBook => "o".to_string(),
            MarketType::PublicTrades => "pt".to_string(),
            MarketType::Ticker => "t".to_string(),
        }
    }
}

#[derive(Debug,PartialEq)]
pub enum InstrumentType {
    /// Futures contract: BASE-QUOTE-EXPIRY (e.g., BTC-USD-20250528)
    /// Expiry format in [`STANDARD_DATE_FORMAT`]
    Future(String, String, String),
    /// Options contract: BASE-QUOTE-EXPIRY-STRIKE-OPTIONKIND (e.g., BTC-USD-20250528-19000-C)
    /// Expiry format in [`STANDARD_DATE_FORMAT`]
    Option(String, String, String, u64, OptionKind),
    /// Spot trading pair: BASE-QUOTE (e.g., BTC-USD)
    Spot(String, String),
    /// Perpetual contract: BASE-QUOTE (e.g., BTC-USD)
    Perpetual(String, String),
}


impl ToString for InstrumentType {
    /// Converts an `InstrumentType` to its string representation
    fn to_string(&self) -> String {
        match self {
            InstrumentType::Future(base, quote, expiry) => format!("f.{}-{}-{}", base, quote, expiry),
            InstrumentType::Option(base, quote, expiry,strike, kind) => {
                match normalize_expiry(expiry) {
                    Some(normalized_expiry) => format!("o.{}-{}-{}-{}-{}", base, quote, normalized_expiry, strike, kind.to_string()),
                    None => "".to_string()
                }
            },
            InstrumentType::Spot(base, quote) => format!("s.{base}-{quote}"),
            InstrumentType::Perpetual(base, quote) => format!("p.{base}-{quote}"),
        }
    }
}

/// Represents an option kind (Call or Put)
#[derive(Debug, PartialEq)]
pub enum OptionKind {
    Call,
    Put
}

impl ToString for OptionKind {
    /// Converts an `OptionKind` to its string representation
    fn to_string(&self) -> String {
        match self {
            OptionKind::Call => "C".to_string(),
            OptionKind::Put => "P".to_string(),
        }
    }
}

impl OptionKind {
    /// Converts a string to an `OptionKind`
    fn from_str(option_kind: &str) -> Option<Self> {
        match option_kind.to_lowercase().as_str() {
            "call" | "c"  => Some(OptionKind::Call),
            "put" | "p" => Some(OptionKind::Put),
            _ => None,
        }
    }
}


/// Represents a trading instrument with details about its market and type
#[derive(Debug, PartialEq)]
pub struct Instrument {
    exchange: Exchange,
    market_type: MarketType,
    instrument_type: InstrumentType
}
impl Instrument {
    /// Creates a new `Instrument`
    pub fn new(exchange: Exchange, market_type: MarketType, instrument_type: InstrumentType) -> Self {
        Self {
            exchange,
            market_type,
            instrument_type
        }
    }
}
impl ToString for Instrument {
    //// Converts an `Instrument` to its standard string representation
    fn to_string(&self) -> String {
        format!("{}.{}.{}", self.market_type.to_string(), self.instrument_type.to_string(), self.exchange.to_string())
    }
}

/// Trait for handling exchange-specific operations
pub trait ExchangeHandler {
    /// Validate if market type is supported by exchange
    /// Validate if the instrument name is valid for the particular exchange
    /// Validate if the instrument type is supported by the exchange
    /// Return normalized instrument if all validations pass
    fn normalize(&self, market_type: MarketType, instrument_name: String) -> Option<Instrument>;

    /// Validate if market type is supported by exchange
    /// Validate if the instrument type is valid for the particular exchange
    /// Return the exchange specific instrument name if all validations pass
    fn denormalize(&self, instrument: Instrument) -> Option<String>;

    // Transform an existing normalized instrument-name to an Instrument
    /// Validate if market type is supported by exchange
    /// Validate if the instrument type is valid for the particular exchange
    /// Return the exchange specific instrument name if all validations pass
    // fn denormalize_from_str(&self, instrument_name: String) -> Option<String>;

    /// An optional function where you can add validation for supported MarketType
    fn market_type_validator(market_type: &MarketType) -> bool {
        let _ = market_type;
        true
    }

    /// An optional function where you can add validation for supported InstrumentType
    fn instrument_type_validator(instrument_type: &InstrumentType) -> bool {
        let _ = instrument_type;
        true
    }
}

/// Parses and normalizes an expiry date
fn normalize_expiry(date_str: &str) -> Option<String> {
    let date_time = if let Ok(date) = NaiveDate::parse_from_str(date_str, "%d%b%y") {
        let date = date.and_time(NaiveTime::from_hms_opt(0,0,0)?);
        let t  = Utc.from_utc_datetime(&date);
        Some(t)
        // Some(Utc.from_utc_date(&date).and_hms(0, 0, 0).timestamp_millis())
    } else if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y%m%d") {
        let date = date.and_time(NaiveTime::from_hms_opt(0,0,0)?);
        let t  = Utc.from_utc_datetime(&date);
        Some(t)
    }else if let Ok(date) = NaiveDate::parse_from_str(date_str, "%d%b%Y") {
        let date = date.and_time(NaiveTime::from_hms_opt(0,0,0)?);
        let t  = Utc.from_utc_datetime(&date);
        Some(t)
    } else {
        error!("normalize_expiry::Failed to parse expiry date: {}", date_str);
        None
    };

    match date_time {
        Some(dt) => Some(format_expiry_date(dt, STANDARD_DATE_FORMAT)),
        None => None
    }
}

/// Parse and denomalize standard expiry format to desired format
fn denormalize_expiry(date_str: &str, format: &str) -> String {
    let parse_expiry = parse_expiry_date(date_str, STANDARD_DATE_FORMAT);
    match parse_expiry {
        Some(utc) => {
            format_expiry_date(utc, format).to_uppercase()
        },
        None => {
            error!("denormalize_expiry::Failed to parse expiry date: {}", date_str);
            "".to_string()
        }
    }
}

/// Parses an expiry date string to `DateTime<Utc>`
fn parse_expiry_date(date_str: &str, format: &str) -> Option<DateTime<Utc>> {
    if let Ok(date) = NaiveDate::parse_from_str(date_str, format) {
        let date = date.and_time(NaiveTime::from_hms_opt(0,0,0)?);
        let t  = Utc.from_utc_datetime(&date);
        return Some(t);
    }
    error!("parse_expiry_date::Failed to parse expiry date: {}", date_str);
    None
}

/// Formats a given `DateTime<Utc>` into a string using the specified format
fn format_expiry_date(date: DateTime<Utc>, format: &str) -> String {
    date.format(format).to_string().to_uppercase()
}

#[cfg(test)]
mod test {
    use chrono::{Utc, TimeZone};

    use crate::parse_expiry_date;

    #[test]
    fn test_parse_expiry_date() {
        let date_str = "28MAR25";
        let format = "%d%b%y";
        println!("{}", parse_expiry_date(date_str, format).unwrap());
    }

    #[test]
    fn test_format_expiry_date() {
        let date = Utc.with_ymd_and_hms(2025, 3, 28, 0, 0, 0).unwrap();
        let format = "%d%b%y";
        println!("{}", crate::format_expiry_date(date, format));
    }
}