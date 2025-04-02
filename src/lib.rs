use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use tracing::error;
use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};
use thiserror::Error;

/// Module containing exchange-related definitions
pub mod exchange;

/// Standard date format for expiry parsing
const STANDARD_DATE_FORMAT: &str = "%Y%m%d";
const LOG_CTX: &str = "normify#lib";

/// Error types for instrument operations
#[derive(Error, Debug)]
pub enum InstrumentError {
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
    
    #[error("Unsupported by exchange: {0}")]
    UnsupportedByExchange(String),
    
    #[error("Invalid date format: {0}")]
    InvalidDate(String),
    
    #[error("Parsing error: {0}")]
    ParseError(String),
}

/// Result type for instrument operations
pub type InstrumentResult<T> = Result<T, InstrumentError>;

/// Parse a standard format string into an Instrument
/// Standard instrument format: <market-type>.<instrument-kind>.<instrument-name>.<exchange>
/// Example: o.p.BTC-USD.deribit
pub fn parse_standard_format(instrument_str: &str) -> InstrumentResult<Instrument> {
    let parts: Vec<&str> = instrument_str.split('.').collect();
    
    match parts.as_slice() {
        [market_type, instrument_kind, instrument_name, exchange] => {
            // Parse exchange and market type once
            let exchange = Exchange::try_from(*exchange)
                .map_err(|e| InstrumentError::ParseError(e))?;
            
            let market_type = MarketType::try_from(*market_type)
                .map_err(|e| InstrumentError::ParseError(e.to_string()))?;
            
            // Parse instrument type
            let instrument_type = InstrumentType::from_str(instrument_kind, instrument_name)
                .ok_or_else(|| InstrumentError::InvalidFormat(
                    format!("Invalid instrument format: {instrument_kind}.{instrument_name}")
                ))?;
            
            let instrument = Instrument {
                exchange,
                market_type,
                instrument_type,
            };
            
            // Validate by attempting to denormalize
            let handler = exchange.handler();
            if handler.denormalize(&instrument).is_some() {
                Ok(instrument)
            } else {
                Err(InstrumentError::UnsupportedByExchange(
                    format!("Instrument not supported by {}", exchange)
                ))
            }
        },
        _ => Err(InstrumentError::InvalidFormat(
            format!("Invalid instrument format: {}", instrument_str)
        )),
    }
}

/// Transform a standard string format to an exchange specific instrument name
pub fn to_exchange_format(instrument_str: &str) -> Option<String> {
    match parse_standard_format(instrument_str) {
        Ok(instrument) => instrument.exchange.handler().denormalize(&instrument),
        Err(err) => {
            error!(name: LOG_CTX, "to_exchange_format error: {}", err);
            None
        }
    }
}

/// Represents different exchanges
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Exchange {
    Deribit,
    Dydx,
    Derive,
    Paradex,
    Aevo
}

impl Display for Exchange {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Exchange::Deribit => "deribit",
            Exchange::Dydx => "dydx",
            Exchange::Derive => "derive",
            Exchange::Paradex => "paradex",
            Exchange::Aevo => "aevo",
        })
    }
}

impl TryFrom<&str> for Exchange {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // Avoid allocation by using match directly on lowercase comparison
        match value.trim() {
            s if s.eq_ignore_ascii_case("deribit") => Ok(Exchange::Deribit),
            s if s.eq_ignore_ascii_case("dydx") => Ok(Exchange::Dydx),
            s if s.eq_ignore_ascii_case("derive") => Ok(Exchange::Derive),
            s if s.eq_ignore_ascii_case("paradex") => Ok(Exchange::Paradex),
            s if s.eq_ignore_ascii_case("aevo") => Ok(Exchange::Aevo),
            _ => Err(format!("Invalid exchange name: {}", value)),
        }
    }
}

impl Exchange {
    /// Returns the appropriate exchange handler
    pub fn handler(&self) -> &'static dyn ExchangeHandler {
        // Static handlers avoid Box allocation
        match self {
            Exchange::Deribit => &exchange::deribit::DERIBIT_HANDLER,
            Exchange::Dydx => &exchange::dydx::DYDX_HANDLER,
            Exchange::Derive => &exchange::derive::DERIVE_HANDLER,
            Exchange::Paradex => &exchange::paradex::PARADEX_HANDLER,
            Exchange::Aevo => &exchange::aevo::AEVO_HANDLER,
        }
    }
}

/// Represents different market types
#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub enum MarketType {
    OrderBook,
    PublicTrade,
    Ticker,
    Funding,
}

impl Display for MarketType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            MarketType::OrderBook => "o",
            MarketType::PublicTrade => "p",
            MarketType::Ticker => "t",
            MarketType::Funding => "f",
        })
    }
}

impl TryFrom<&str> for MarketType {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // Avoid allocation by using match directly on lowercase comparison
        match value.trim() {
            s if s.eq_ignore_ascii_case("o") || s.eq_ignore_ascii_case("orderbook") => 
                Ok(MarketType::OrderBook),
            s if s.eq_ignore_ascii_case("p") || s.eq_ignore_ascii_case("publictrade")
                || s.eq_ignore_ascii_case("trade") => 
                Ok(MarketType::PublicTrade),
            s if s.eq_ignore_ascii_case("t") || s.eq_ignore_ascii_case("ticker") => 
                Ok(MarketType::Ticker),
            s if s.eq_ignore_ascii_case("f") || s.eq_ignore_ascii_case("funding") => 
                Ok(MarketType::Funding),
            _ => Err("Invalid market type"),
        }
    }
}

/// Represents different instrument types with their specificities
#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub enum InstrumentType {
    /// Futures contract: BASE-QUOTE-EXPIRY (e.g., BTC-USD-20250528)
    Future {
        base: Currency,
        quote: Currency,
        expiry: Cow<'static, str>,
    },
    
    /// Options contract: BASE-QUOTE-EXPIRY-STRIKE-OPTIONKIND (e.g., BTC-USD-20250528-19000-C)
    Option {
        base: Currency,
        quote: Currency,
        expiry: Cow<'static, str>,
        strike: u64,
        kind: OptionKind,
    },
    
    /// Spot trading pair: BASE-QUOTE (e.g., BTC-USD)
    Spot {
        base: Currency,
        quote: Currency,
    },
    
    /// Perpetual contract: BASE-QUOTE (e.g., BTC-USD)
    Perpetual {
        base: Currency,
        quote: Currency,
    },
}

impl Display for InstrumentType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            InstrumentType::Future { base, quote, expiry } => 
                write!(f, "f.{}-{}-{}", base.as_ref(), quote.as_ref(), expiry),
                
            InstrumentType::Option { base, quote, expiry, strike, kind } => {
                match parse_expiry_date(expiry, STANDARD_DATE_FORMAT) {
                    Some(date) => write!(f, "o.{}-{}-{}-{}-{}", 
                                        base.as_ref(), quote.as_ref(), 
                                        format_expiry_date(date, STANDARD_DATE_FORMAT), 
                                        strike, kind),
                    None => Err(fmt::Error),
                }
            },
            
            InstrumentType::Spot { base, quote } => 
                write!(f, "s.{}-{}", base.as_ref(), quote.as_ref()),
                
            InstrumentType::Perpetual { base, quote } => 
                write!(f, "p.{}-{}", base.as_ref(), quote.as_ref()),
        }
    }
}

impl InstrumentType {
    /// Create an InstrumentType from string components
    /// 
    /// * `kind` - The kind of instrument (e.g., "future", "option", "spot")
    /// * `instrument_name` - The full name of the instrument (e.g., "BTC-USD-202306")
    pub fn from_str(kind: &str, instrument_name: &str) -> Option<Self> {
        // Split the instrument name once
        let parts: Vec<&str> = instrument_name.split('-').collect();
        
        // Match on kind with case-insensitive comparison but no allocation
        match kind.trim() {
            k if k.eq_ignore_ascii_case("o") || k.eq_ignore_ascii_case("option") => {
                // Parse option details
                if let [base, quote, expiry, strike, option_kind] = parts.as_slice() {
                    let option_kind = OptionKind::try_from(*option_kind).ok()?;
                    let strike = strike.parse::<u64>().ok()?;
                    
                    Some(InstrumentType::Option { 
                        base: Currency::new(Cow::Owned(base.to_string())),
                        quote: Currency::new(Cow::Owned(quote.to_string())),
                        expiry: Cow::Owned(expiry.to_string()),
                        strike,
                        kind: option_kind,
                    })
                } else {
                    None
                }
            },
            
            k if k.eq_ignore_ascii_case("f") || k.eq_ignore_ascii_case("future") => {
                if let [base, quote, expiry] = parts.as_slice() {
                    Some(InstrumentType::Future {
                        base: Currency::new(Cow::Owned(base.to_string())),
                        quote: Currency::new(Cow::Owned(quote.to_string())),
                        expiry: Cow::Owned(expiry.to_string()),
                    })
                } else {
                    None
                }
            },
            
            k if k.eq_ignore_ascii_case("p") || k.eq_ignore_ascii_case("perpetual") => {
                if let [base, quote] = parts.as_slice() {
                    Some(InstrumentType::Perpetual {
                        base: Currency::new(Cow::Owned(base.to_string())),
                        quote: Currency::new(Cow::Owned(quote.to_string())),
                    })
                } else {
                    None
                }
            },
            
            k if k.eq_ignore_ascii_case("s") || k.eq_ignore_ascii_case("spot") => {
                if let [base, quote] = parts.as_slice() {
                    Some(InstrumentType::Spot {
                        base: Currency::new(Cow::Owned(base.to_string())),
                        quote: Currency::new(Cow::Owned(quote.to_string())),
                    })
                } else {
                    None
                }
            },
            
            _ => None,
        }
    }

    /// Get base currency
    pub fn base(&self) -> &str {
        match self {
            InstrumentType::Future { base, .. } => base.as_ref(),
            InstrumentType::Option { base, .. } => base.as_ref(),
            InstrumentType::Spot { base, .. } => base.as_ref(),
            InstrumentType::Perpetual { base, .. } => base.as_ref(),
        }
    }

    /// Get quote currency
    pub fn quote(&self) -> &str {
        match self {
            InstrumentType::Future { quote, .. } => quote.as_ref(),
            InstrumentType::Option { quote, .. } => quote.as_ref(),
            InstrumentType::Spot { quote, .. } => quote.as_ref(),
            InstrumentType::Perpetual { quote, .. } => quote.as_ref(),
        }
    }
}

/// Represents an option kind (Call or Put)
#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash)]
pub enum OptionKind {
    Call,
    Put,
}

impl Display for OptionKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            OptionKind::Call => "C",
            OptionKind::Put => "P",
        })
    }
}

impl TryFrom<&str> for OptionKind {
    type Error = String;
    
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            s if s.eq_ignore_ascii_case("c") || s.eq_ignore_ascii_case("call") => 
                Ok(OptionKind::Call),
            s if s.eq_ignore_ascii_case("p") || s.eq_ignore_ascii_case("put") => 
                Ok(OptionKind::Put),
            _ => Err(format!("Invalid option kind: {}", value)),
        }
    }
}

/// Represents a trading instrument with details about its market and type
#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct Instrument {
    pub exchange: Exchange,
    pub market_type: MarketType,
    pub instrument_type: InstrumentType,
}

impl Instrument {
    /// Creates a new `Instrument`
    pub fn new(exchange: Exchange, market_type: MarketType, instrument_type: InstrumentType) -> Self {
        Self {
            exchange,
            market_type,
            instrument_type,
        }
    }

    pub fn is_expired(&self) -> bool {
        match &self.instrument_type {
            InstrumentType::Option {expiry,..} | InstrumentType::Future {expiry,..} => {
                match is_date_expired(expiry) {
                    Ok(expired) => expired,
                    Err(err) => {
                        error!(name: LOG_CTX, "{}", err);
                        false
                    }
                }
            },
            _ => false
        }
    }
}

impl Display for Instrument {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.market_type, self.instrument_type, self.exchange)
    }
}

/// Trait for handling exchange-specific operations
pub trait ExchangeHandler {
    /// Normalize an exchange-specific instrument name to our standard format
    /// Returns None if the instrument is not valid for this exchange
    fn normalize(&self, market_type: MarketType, instrument_name: &str) -> Option<Instrument>;

    /// Convert a standard instrument to an exchange-specific format
    /// Returns None if the instrument is not valid for this exchange
    fn denormalize(&self, instrument: &Instrument) -> Option<String>;

    /// Check if market type is supported by this exchange
    fn supports_market_type(&self, market_type: &MarketType) -> bool {
        let _ = market_type;
        true
    }

    /// Check if instrument type is supported by this exchange
    fn supports_instrument_type(&self, instrument_type: &InstrumentType) -> bool {
        let _ = instrument_type;
        true
    }
}

/// Date handling functions

/// Parses and normalizes an expiry date to standard format
fn normalize_expiry(date_str: &str) -> Option<String> {
    // Use a single vector of formats to try, avoiding repetitive code
    let formats = ["%d%b%y", STANDARD_DATE_FORMAT, "%d%b%Y"];
    
    for format in &formats {
        if let Some(dt) = parse_expiry_date(date_str, format) {
            return Some(format_expiry_date(dt, STANDARD_DATE_FORMAT));
        }
    }
    
    error!(name: LOG_CTX, "normalize_expiry: Failed to parse expiry date: {}", date_str);
    None
}

/// Parse and normalize standard expiry format to desired format
fn denormalize_expiry(date_str: &str, format: &str) -> String {
    match parse_expiry_date(date_str, STANDARD_DATE_FORMAT) {
        Some(utc) => format_expiry_date(utc, format).to_uppercase(),
        None => {
            error!(name: LOG_CTX, "denormalize_expiry: Failed to parse expiry date: {}", date_str);
            String::new()
        }
    }
}

/// Parses an expiry date string to `DateTime<Utc>`
fn parse_expiry_date(date_str: &str, format: &str) -> Option<DateTime<Utc>> {
    NaiveDate::parse_from_str(date_str, format)
        .ok()
        .and_then(|date| {
            NaiveTime::from_hms_opt(0, 0, 0).map(|time| date.and_time(time))
        })
        .map(|naive| Utc.from_utc_datetime(&naive))
}

/// Formats a given `DateTime<Utc>` into a string using the specified format
fn format_expiry_date(date: DateTime<Utc>, format: &str) -> String {
    date.format(format).to_string()
}

pub fn is_date_expired(date_str: &str) -> Result<bool, String> {
    // Parse the input date string
    let naive_date = match NaiveDate::parse_from_str(date_str, STANDARD_DATE_FORMAT) {
        Ok(d) => d,
        Err(err) => return Err(format!("{}", err))
    };
    
    // Create a time at 23:59:59.999 (end of day with millisecond precision)
    let end_of_day_time = NaiveTime::from_hms_milli_opt(23, 59, 59, 999).unwrap();
    
    // Combine date and time
    let naive_datetime = naive_date.and_time(end_of_day_time);
    
    // Convert to DateTime<Utc>
    let expiration_date = Utc.from_utc_datetime(&naive_datetime);
    
    // Get current UTC time
    let now = Utc::now();
    
    // Compare: if now is after expiration date, then the date is expired
    Ok(now > expiration_date)
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
pub struct Currency(Cow<'static, str>);

impl Currency {
    pub fn new(symbol: impl Into<Cow<'static, str>>) -> Self {
        let symbol = symbol.into();
        let upper = symbol.to_uppercase();
        
        // Check if it's already uppercase
        if symbol == upper {
            return Currency(symbol);
        }
        
        // For common codes, return static references
        match upper.as_ref() {
            "BTC" => Currency(Cow::Borrowed("BTC")),
            "USD" => Currency(Cow::Borrowed("USD")),
            "ETH" => Currency(Cow::Borrowed("ETH")),
            "SOL" => Currency(Cow::Borrowed("SOL")),
            "USDC" => Currency(Cow::Borrowed("USDC")),
            // Add other common currencies
            _ => Currency(Cow::Owned(upper))
        }
    }
}

impl AsRef<str> for Currency {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
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
    use crate::parse_standard_format;


    #[test]
    fn test_denormalize_option() {
        let standard_format = "o.o.BTC-USD-20250528-100000-C.deribit";
        let denormalized_instrument = parse_standard_format(standard_format);
        assert!(denormalized_instrument.is_ok());
    }
    #[test]
    fn test_denormalize_future() {
        let standard_format = "o.f.BTC-USD-20250528.deribit";
        let denormalized_instrument = parse_standard_format(standard_format);
        assert!(denormalized_instrument.is_ok());
    }
    #[test]
    fn test_denormalize_perpetual() {
        let standard_format = "o.p.BTC-USD.deribit";
        let denormalized_instrument = parse_standard_format(standard_format);
        assert!(denormalized_instrument.is_ok());
    }
    #[test]
    fn test_denormalize_spot() {
        let standard_format = "o.s.BTC-USD.deribit";
        let denormalized_instrument = parse_standard_format(standard_format);
        assert!(denormalized_instrument.is_ok());
    }
    #[test]
    fn test_denormalize_invalid() {
        let standard_format = "o.o.BTC-USD.deribit";
        let denormalized_instrument = parse_standard_format(standard_format);
        assert!(denormalized_instrument.is_ok());
    }
}

// For testing with different dates
#[test]
fn test_multiple_dates() {
    let now = Utc::now();
    println!("Current UTC time: {}", now);
    
    let dates = vec![
        "20250319", // Yesterday (if today is 2025-03-20)
        "20250320", // Today
        "20250321", // Tomorrow
    ];
    println!("Time now: {}", Utc::now());
    for date in dates {
        let is_expired = is_date_expired(date).unwrap();
        println!("Date {} is {}", date, if is_expired { "expired" } else { "not expired" });
    }
}