use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use tracing::error;

/// Module containing exchange-related definitions
pub mod exchange;

/// Standard date format for expiry parsing
const STANDARD_DATE_FORMAT: &str = "%Y%m%d";

/// Standard instrument format o.p.<instrument-name>.exchange , eg. o.p.BTC-USD.deribit.
/// Transform a standard string format to an `Instrument` struct.
/// This will only  work for instrument that are supported by particular exchange 
pub fn transform_from_standard_str_format(instrument_name: &str) -> Option<Instrument> {
    let parts: Vec<&str> = instrument_name.split('.').collect();
    match parts.as_slice() {
        [market_type, instrument_kind, instrument_name, exchange] => {
            if let (Ok(mt), Ok(exc)) = ((*market_type).try_into(), (*exchange).try_into()) {
                let instrument = Instrument {
                    exchange: exc,
                    market_type: mt,
                    instrument_type: InstrumentType::from(instrument_kind, instrument_name)?
                };
                // Why denormalize?
                // This is to validate if MarketType and InstrumentType is supported by the exchange.
                if let Some(_de) = exc.wrap().denormalize(instrument.clone()){
                    return Some(instrument)
                }
            }
            error!("denormalize_from_standard_str_format::Exchange and MarketType are not supported: {}:{}", exchange, market_type);
            None
        },
        _ => {
            error!("denormalize_from_standard_str_format::Unexpected instrument format: {:?}", instrument_name);
            return None;
        }
    }
}

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

impl TryFrom<&str> for Exchange {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim().to_lowercase().as_str() {
            "deribit" => Ok(Exchange::Deribit),
            "dydx" => Ok(Exchange::Dydx),
            "derive" => Ok(Exchange::Derive),
            "paradex" => Ok(Exchange::Paradex),
            _ => Err(format!("Invalid exchange name: {}", value)),
        }
    }
}
impl Exchange {

    fn wrap(&self) -> Box<dyn ExchangeHandler> {
        match self {
            Exchange::Deribit => Box::new(exchange::deribit::DeribitHandler(self.clone())),
            Exchange::Dydx => Box::new(exchange::dydx::DydxHandler(self.clone())),
            Exchange::Derive => Box::new(exchange::derive::DeriveHandler(self.clone())),
            Exchange::Paradex => Box::new(exchange::paradex::ParadexHandler(self.clone()))
        }
    }
}

/// Represents different market types
#[derive(Debug, PartialEq, Clone)]
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

impl TryFrom<&str> for MarketType {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "o" | "orderbook" => Ok(MarketType::OrderBook),
            "pt" | "publictrade" | "trade" => Ok(MarketType::PublicTrades),
            "t" | "ticker" => Ok(MarketType::Ticker),
            _ => Err("Invalid market type"),
        }
    }
}

#[derive(Debug,PartialEq, Clone)]
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

impl InstrumentType {
    /// * `kind` - The kind of instrument (e.g., "future", "option", "spot")
    /// * `instrument_name` - The full name of the instrument (e.g., "BTC-USD-202306")
    /// 
    /// Returns an `Some(InstrumentType)` if the string is valid for the given kind, otherwise returns `None`.
    /// 
    pub fn from(kind: &str, instrument_name: &str) -> Option<Self> {
        match kind {
            "o" | "option" => {
                let parts: Vec<&str> = instrument_name.split('-').collect();
                if let [base, quote, expiry, strike, kind] = parts.as_slice() {
                    let option_kind = (*kind).try_into().ok()?;
                    let strike = strike.parse::<u64>().ok()?;
                    Some(InstrumentType::Option(
                        base.to_string(),
                        quote.to_string(),
                        expiry.to_string(),
                        strike,
                        option_kind,
                    ))
                } else {
                    None
                }
            }
            "f" | "future" => {
                let parts: Vec<&str> = instrument_name.split('-').collect();
                if let [base, quote, expiry] = parts.as_slice() {
                    Some(InstrumentType::Future(
                        base.to_string(),
                        quote.to_string(),
                        expiry.to_string(),
                    ))
                } else {
                    None
                }
            }
            "p" | "perpetual" => {
                let parts: Vec<&str> = instrument_name.split('-').collect();
                if let [base, quote] = parts.as_slice() {
                    Some(InstrumentType::Perpetual(
                        base.to_string(),
                        quote.to_string(),
                    ))
                } else {
                    None
                }
            }
            "s" | "spot" => {
                let parts: Vec<&str> = instrument_name.split('-').collect();
                if let [base, quote] = parts.as_slice() {
                    Some(InstrumentType::Spot(base.to_string(), quote.to_string()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}


/// Represents an option kind (Call or Put)
#[derive(Debug, PartialEq, Clone)]
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

impl TryFrom<&str> for OptionKind {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "c" | "call" => Ok(OptionKind::Call),
            "p" | "put" => Ok(OptionKind::Put),
            _ => Err(format!("Invalid option kind: {}", value)),
        }
    }
}


/// Represents a trading instrument with details about its market and type
#[derive(Debug, PartialEq, Clone)]
pub struct Instrument {
    pub exchange: Exchange,
    pub market_type: MarketType,
    pub instrument_type: InstrumentType
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
    fn market_type_validator(&self, market_type: &MarketType) -> bool {
        let _ = market_type;
        true
    }

    /// An optional function where you can add validation for supported InstrumentType
    fn instrument_type_validator(&self,instrument_type: &InstrumentType) -> bool {
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

#[cfg(test)]
mod test_denormalize {
    use super::transform_from_standard_str_format;

    #[test]
    fn test_denormalize_option() {
        let standard_format = "o.o.BTC-USD-20250528-100000-C.deribit";
        let denormalized_instrument = transform_from_standard_str_format(standard_format);
        assert!(denormalized_instrument.is_some());
    }
    #[test]
    fn test_denormalize_future() {
        let standard_format = "o.f.BTC-USD-20250528.deribit";
        let denormalized_instrument = transform_from_standard_str_format(standard_format);
        assert!(denormalized_instrument.is_some());
    }
    #[test]
    fn test_denormalize_perpetual() {
        let standard_format = "o.p.BTC-USD.deribit";
        let denormalized_instrument = transform_from_standard_str_format(standard_format);
        assert!(denormalized_instrument.is_some());
    }
    #[test]
    fn test_denormalize_spot() {
        let standard_format = "o.s.BTC-USD.deribit";
        let denormalized_instrument = transform_from_standard_str_format(standard_format);
        assert!(denormalized_instrument.is_some());
    }
    #[test]
    fn test_denormalize_invalid() {
        let standard_format = "o.o.BTC-USD.deribit";
        let denormalized_instrument = transform_from_standard_str_format(standard_format);
        assert!(denormalized_instrument.is_none());
    }
}