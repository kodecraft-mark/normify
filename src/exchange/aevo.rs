use tracing::error;
use std::borrow::Cow;

use crate::{denormalize_expiry, normalize_expiry, parse_expiry_date, Currency, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType, OptionKind};

const LOG_CTX: &str = "normify::exchange#aevo";
const DEFAULT_QUOTE_CURRENCY: &str = "usdc";
const DEFAULT_EXPIRY_FORMAT: &str = "%d%b%y";

pub struct Aevohandler;
        
// Create a static instance to avoid allocations
pub static AEVO_HANDLER: Aevohandler = Aevohandler;

impl ExchangeHandler for Aevohandler {
    fn normalize(&self, market_type: MarketType, instrument_name: &str) -> Option<Instrument> {
        
        if !self.supports_market_type(&market_type) {
            error!(name: LOG_CTX, "denormalize::Market Type is unsupported: {:?}", market_type);
            return None;
        }
        // Split the instrument name into parts
        let parts: Vec<&str> = instrument_name.split('-').collect();
    
        match parts.as_slice() {
            // Perpetual: e.g., BTC-PERPETUAL or SOL_USDC-PERPETUAL (Non USD quote)
            [base_quote, perpetual] if perpetual.eq_ignore_ascii_case("perp") => {
                // Use split_once to avoid additional allocations
                let (base, quote) = if let Some((b, q)) = base_quote.split_once('_') {
                    (b, q)
                } else {
                    (*base_quote, DEFAULT_QUOTE_CURRENCY)
                };
                
                Some(Instrument::new(
                    Exchange::Aevo, 
                    market_type, 
                    InstrumentType::Perpetual {
                        base: Currency::new(Cow::Owned(base.to_string())), 
                        quote: Currency::new(Cow::Owned(quote.to_string())), 
                    }
                ))
            }
    
            // Option: e.g., BTC-28MAR25-100000-C
            [base, expiry, strike_str, kind_str] => {
                // Validate the expiry date
                if parse_expiry_date(expiry, DEFAULT_EXPIRY_FORMAT).is_none() {
                    error!(name: LOG_CTX, "normalize::Invalid expiry date format: {}", expiry);
                    return None;
                }
                
                // Parse strike price
                let strike = match strike_str.parse::<u64>() {
                    Ok(s) => s,
                    Err(_) => {
                        error!(name: LOG_CTX, "normalize::Invalid strike price: {}", strike_str);
                        return None;
                    }
                };
                
                // Parse option kind
                let kind = match OptionKind::try_from(*kind_str) {
                    Ok(k) => k,
                    Err(e) => {
                        error!(name: LOG_CTX, "normalize::Invalid option kind: {}", e);
                        return None;
                    }
                };
                
                let normalized_expiry = normalize_expiry(expiry)?;
                
                Some(Instrument::new(
                    Exchange::Aevo,
                    market_type,
                    InstrumentType::Option {
                        base: Currency::new(Cow::Owned(base.to_string())), 
                        quote: Currency::new(Cow::Owned(DEFAULT_QUOTE_CURRENCY.to_string())),
                        expiry: Cow::Owned(normalized_expiry), 
                        strike, 
                        kind
                    }
                ))
            }
    
            // No matching format
            _ => {
                error!(name: LOG_CTX, "normalize::Unexpected instrument format: {:?}", instrument_name);
                None
            }
        }
    }

    fn denormalize(&self, instrument: &Instrument) -> Option<String> {
        // Check if this is the right exchange handler
        if instrument.exchange != Exchange::Aevo {
            error!(name: LOG_CTX, "denormalize::Attempted to use Aevo handler for {:?}", instrument.exchange);
            return None;
        }

        if !self.supports_instrument_type(&instrument.instrument_type) {
            error!(name: LOG_CTX, "denormalize::Instrument Type for {:?} is unsupported", instrument.instrument_type);
            return None;
        }
        if !self.supports_market_type(&instrument.market_type) {
            error!(name: LOG_CTX, "denormalize::Market Type for {:?} is unsupported",instrument.market_type);
            return None;
        }
        
        match &instrument.instrument_type {
            
            InstrumentType::Option { base, quote: _, expiry, strike, kind } => {
                let denormalized_expiry = denormalize_expiry(expiry, DEFAULT_EXPIRY_FORMAT);
                Some(format!("{}-{}-{}-{}", 
                    base.as_ref(), 
                    denormalized_expiry, 
                    strike, 
                    kind.to_string()))
            },
            
            InstrumentType::Perpetual { base, .. } => {
                Some(format!("{}-PERP", base.as_ref()))
            },
            _ => None
        }
    }

    fn supports_market_type(&self, market_type: &MarketType) -> bool {
        matches!(market_type, MarketType::OrderBook) || matches!(market_type, MarketType::Ticker)
    }

    fn supports_instrument_type(&self, instrument_type: &InstrumentType) -> bool {
        matches!(instrument_type, InstrumentType::Perpetual { .. }) || matches!(instrument_type, InstrumentType::Option { .. })
    }
}

#[cfg(test)]
mod deribit_normalize_tests{
    use std::borrow::Cow;

    use crate::{exchange::aevo::Aevohandler, Currency, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType, OptionKind};


    #[test]
    fn test_normalize_option() {
        let instrument_name = "BTC-28MAR25-100000-C";
        let exchange = Aevohandler;
        let market_type = MarketType::OrderBook;
        let expected_instrument = Instrument::new(
            Exchange::Aevo, 
            market_type, 
            InstrumentType::Option{
                base: Currency::new(Cow::Borrowed("BTC")), 
                quote: Currency::new(Cow::Borrowed("USDC")), 
                expiry: Cow::Borrowed("20250328"),
                strike: 100000, 
                kind: OptionKind::Call});
        let result = exchange.normalize(MarketType::OrderBook,instrument_name);
        println!("{:?}", result);
        assert_eq!(result, Some(expected_instrument));
    }

    #[test]
    fn test_normalize_perpetual1() {
        let instrument_name = "BTC-PERP";
        let exchange = Aevohandler;
        let market_type = MarketType::OrderBook;
        let expected_instrument = Instrument::new(
            Exchange::Aevo, 
            market_type, 
            InstrumentType::Perpetual{
                base: Currency::new(Cow::Borrowed("BTC")), 
                quote: Currency::new(Cow::Borrowed("USDC")), 
            });
        let result = exchange.normalize(MarketType::OrderBook,instrument_name);
        println!("{:?}", result);
        assert_eq!(result, Some(expected_instrument));
    }

    #[test]
    fn test_normalize_unknown() {
        let instrument_name = "BTC-USD-20250528";
        let exchange = Aevohandler;
        assert_eq!(exchange.normalize(MarketType::OrderBook, instrument_name), None);
    }
}

#[cfg(test)]
mod deribit_denormalize_tests{
    use std::borrow::Cow;

    use crate::{exchange::aevo::Aevohandler, Currency, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType, OptionKind};

    #[test]
    fn test_denorm_option() {
        let instrument = Instrument::new(
            Exchange::Aevo, 
            MarketType::OrderBook, 
            InstrumentType::Option{
                base: Currency::new(Cow::Borrowed("BTC")), 
                quote: Currency::new(Cow::Borrowed("USDC")), 
                expiry: Cow::Borrowed("20250328"), 
                strike: 100000, 
                kind: OptionKind::Call});
        let exchange = Aevohandler;
        assert_eq!(exchange.denormalize(&instrument), Some(String::from("BTC-28MAR25-100000-C")));
    }

    #[test]
    fn test_denorm_perp1() {
        let instrument = Instrument::new(
            Exchange::Aevo, 
            MarketType::OrderBook, 
            InstrumentType::Perpetual{
                base: Currency::new(Cow::Borrowed("BTC")), 
                quote: Currency::new(Cow::Borrowed("USDC")), 
            });
        let exchange = Aevohandler;
        assert_eq!(exchange.denormalize(&instrument), Some(String::from("BTC-PERP")));
    }
}