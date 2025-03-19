use std::borrow::Cow;

use tracing::error;

use crate::{denormalize_expiry, normalize_expiry, parse_expiry_date, Currency, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType, OptionKind};

const LOG_CTX: &str = "normify::exchange#derive";
const DEFAULT_QUOTE_CURRENCY: &str = "usd";
const DEFAULT_EXPIRY_FORMAT: &str = "%Y%m%d";
pub struct DeriveHandler;
        
// Create a static instance to avoid allocations
pub static DERIVE_HANDLER: DeriveHandler = DeriveHandler;

impl ExchangeHandler for DeriveHandler {

    fn normalize(&self, market_type: MarketType, instrument_name: &str) -> Option<Instrument> {

    
        let parts: Vec<&str> = instrument_name.split('-').collect();
    
        match parts.as_slice() {
            // Perpetual: e.g., BTC-PERP
            [base, "perp" | "PERP"] => {
                Some(Instrument::new(
                    Exchange::Derive, 
                    market_type, 
                    InstrumentType::Perpetual {
                        base: Currency::new(Cow::Owned(base.to_string())), 
                        quote: Currency::new(Cow::Owned(DEFAULT_QUOTE_CURRENCY.to_string()))
                    }
                ))
            }
    
            // Option: e.g., BTC-20250328-100000-C
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
                    Exchange::Derive,
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

        if instrument.exchange != Exchange::Derive {
            error!(name: LOG_CTX, "denormalize::Attempted to use Derive handler for {:?}", instrument.exchange);
            return None;
        }
        if !self.supports_instrument_type(&instrument.instrument_type) {
            error!(name: LOG_CTX, "denormalize::Instrument Type {:?} is unsupported", &instrument.instrument_type);
            return None;
        }
        match &instrument.instrument_type {
            InstrumentType::Option{base, quote: _, expiry, strike, kind} => {
                let denormalize_expiry = denormalize_expiry(&expiry, DEFAULT_EXPIRY_FORMAT);
                Some(format!("{}-{}-{}-{}", base.as_ref(), denormalize_expiry, strike, kind.to_string()))
            },
            InstrumentType::Perpetual{base, quote: _} => Some(format!("{}-PERP", base.as_ref())),
            _ => None
        }
    }

    fn supports_instrument_type(&self, instrument_type: &InstrumentType) -> bool {
        matches!(instrument_type, InstrumentType::Option { base: _, quote: _, expiry: _, strike: _, kind: _ })
        ||
        matches!(instrument_type, InstrumentType::Perpetual { base: _, quote: _ })
    }
}

#[cfg(test)]
mod derive_normalize_tests{
    use std::borrow::Cow;

    use crate::{exchange::derive::DeriveHandler, Currency, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType, OptionKind};
    #[test]
    fn test_normalize_option() {
        let instrument_name = "BTC-20250328-100000-C".to_string();
        let exchange = DeriveHandler;
        let market_type = MarketType::OrderBook;
        let expected_instrument = Instrument::new(
            Exchange::Derive, 
            market_type, 
            InstrumentType::Option{
                base: Currency::new(Cow::Borrowed("BTC")), 
                quote: Currency::new(Cow::Borrowed("USD")), 
                expiry: Cow::Borrowed("20250328"), 
                strike: 100000, 
                kind: OptionKind::Call
            });
        let result = exchange.normalize(MarketType::OrderBook, &instrument_name);
        println!("{:?}", result);
        assert_eq!(result, Some(expected_instrument));
    }

    #[test]
    fn test_normalize_perpetual() {
        let instrument_name = "BTC-PERP".to_string();
        let exchange = DeriveHandler;
        let market_type = MarketType::OrderBook;
        let expected_instrument = Instrument::new(
            Exchange::Derive, 
            market_type, 
            InstrumentType::Perpetual{
                base: Currency::new(Cow::Borrowed("BTC")), 
                quote: Currency::new(Cow::Borrowed("USD"))
            });
        assert_eq!(exchange.normalize(MarketType::OrderBook, &instrument_name), Some(expected_instrument));
    }

    #[test]
    fn test_normalize_unknown() {
        let instrument_name = "BTC-28MAR25-100000-C".to_string();
        let exchange = DeriveHandler;
        assert_eq!(exchange.normalize(MarketType::OrderBook, &instrument_name), None);
    }
}

#[cfg(test)]
mod derive_denormalize_tests{
    use std::borrow::Cow;

    use crate::{exchange::derive::DeriveHandler, Currency, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType, OptionKind};

    #[test]
    fn test_denorm_option() {
        let instrument = Instrument::new(
            Exchange::Derive, 
            MarketType::OrderBook, 
            InstrumentType::Option{
                base: Currency::new(Cow::Borrowed("BTC")), 
                quote: Currency::new(Cow::Borrowed("USD")), 
                expiry: Cow::Borrowed("20250328"), 
                strike: 100000, 
                kind: OptionKind::Call
            });
        let exchange = DeriveHandler;
        assert_eq!(exchange.denormalize(&instrument), Some(String::from("BTC-20250328-100000-C")));
    }

    #[test]
    fn test_denorm_perp() {
        let instrument = Instrument::new(
            Exchange::Derive, 
            MarketType::OrderBook, 
            InstrumentType::Perpetual{
                base: Currency::new(Cow::Borrowed("BTC")), 
                quote: Currency::new(Cow::Borrowed("USD")), 
            });
        let exchange = DeriveHandler;
        assert_eq!(exchange.denormalize(&instrument), Some(String::from("BTC-PERP")));
    }
}