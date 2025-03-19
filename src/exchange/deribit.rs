use tracing::error;
use std::borrow::Cow;

use crate::{denormalize_expiry, normalize_expiry, parse_expiry_date, Currency, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType, OptionKind};

const LOG_CTX: &str = "normify::exchange#deribit";
const DEFAULT_QUOTE_CURRENCY: &str = "usd";
const DEFAULT_EXPIRY_FORMAT: &str = "%d%b%y";

pub struct DeribitHandler;
        
// Create a static instance to avoid allocations
pub static DERIBIT_HANDLER: DeribitHandler = DeribitHandler;

impl ExchangeHandler for DeribitHandler {
    fn normalize(&self, market_type: MarketType, instrument_name: &str) -> Option<Instrument> {
        
        // Split the instrument name into parts
        let parts: Vec<&str> = instrument_name.split('-').collect();
    
        match parts.as_slice() {
            // Perpetual: e.g., BTC-PERPETUAL or SOL_USDC-PERPETUAL (Non USD quote)
            [base_quote, perpetual] if perpetual.eq_ignore_ascii_case("perpetual") => {
                // Use split_once to avoid additional allocations
                let (base, quote) = if let Some((b, q)) = base_quote.split_once('_') {
                    (b, q)
                } else {
                    (*base_quote, DEFAULT_QUOTE_CURRENCY)
                };
                
                Some(Instrument::new(
                    Exchange::Deribit, 
                    market_type, 
                    InstrumentType::Perpetual {
                        base: Currency::new(Cow::Owned(base.to_string())), 
                        quote: Currency::new(Cow::Owned(quote.to_string())), 
                    }
                ))
            }
    
            // Future: e.g., BTC-28MAR25
            [base, expiry] if parse_expiry_date(expiry, DEFAULT_EXPIRY_FORMAT).is_some() => {
                let normalized_expiry = normalize_expiry(expiry)?;
                
                Some(Instrument::new(
                    Exchange::Deribit, 
                    market_type, 
                    InstrumentType::Future {
                        base: Currency::new(Cow::Owned(base.to_string())),  
                        quote: Currency::new(Cow::Owned(DEFAULT_QUOTE_CURRENCY.to_string())),
                        expiry: Cow::Owned(normalized_expiry)
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
                    Exchange::Deribit,
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
    
            // Spot: e.g., BTC_USD
            [spot] => {
                let parts: Vec<&str> = spot.split('_').collect();
                if parts.len() != 2 {
                    error!(name: LOG_CTX, "normalize::Invalid spot format: {}", spot);
                    return None;
                }
                
                Some(Instrument::new(
                    Exchange::Deribit, 
                    market_type, 
                    InstrumentType::Spot {
                        base: Currency::new(Cow::Owned(parts[0].to_string())), 
                        quote: Currency::new(Cow::Owned(parts[1].to_string())), 
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
        if instrument.exchange != Exchange::Deribit {
            error!(name: LOG_CTX, "denormalize::Attempted to use Deribit handler for {:?}", instrument.exchange);
            return None;
        }
        
        match &instrument.instrument_type {
            InstrumentType::Future { base, quote: _, expiry } => {
                let denormalized_expiry = denormalize_expiry(expiry, DEFAULT_EXPIRY_FORMAT);
                Some(format!("{}-{}", base.as_ref(), denormalized_expiry))
            },
            
            InstrumentType::Option { base, quote: _, expiry, strike, kind } => {
                let denormalized_expiry = denormalize_expiry(expiry, DEFAULT_EXPIRY_FORMAT);
                Some(format!("{}-{}-{}-{}", 
                    base.as_ref(), 
                    denormalized_expiry, 
                    strike, 
                    kind.to_string()))
            },
            
            InstrumentType::Spot { base, quote } => {
                Some(format!("{}_{}", base.as_ref(), quote.as_ref()))
            },
            
            InstrumentType::Perpetual { base, quote } => {
                if quote.as_ref().eq_ignore_ascii_case(DEFAULT_QUOTE_CURRENCY) {
                    Some(format!("{}-PERPETUAL", base.as_ref()))
                } else {
                    Some(format!("{}_{}-PERPETUAL", 
                        base.as_ref(), 
                        quote.as_ref()))
                }
            }
        }
    }
}

#[cfg(test)]
mod deribit_normalize_tests{
    use std::borrow::Cow;

    use crate::{exchange::deribit::DeribitHandler, Currency, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType, OptionKind};

    #[test]
    fn test_normalize_future() {
        let instrument_name = "BTC-28MAR25";
        let exchange = DeribitHandler;
        let market_type = MarketType::OrderBook;
        let expected_instrument = Instrument::new(
            Exchange::Deribit, 
            market_type, 
            InstrumentType::Future{
                base: Currency::new(Cow::Borrowed("BTC")), 
                quote: Currency::new(Cow::Borrowed("USD")), 
                expiry: Cow::Borrowed("20250328")
            });
        let result = exchange.normalize(MarketType::OrderBook,instrument_name);
        println!("{:?}", result);
        assert_eq!(result, Some(expected_instrument));
    }

    #[test]
    fn test_normalize_option() {
        let instrument_name = "BTC-28MAR25-100000-C";
        let exchange = DeribitHandler;
        let market_type = MarketType::OrderBook;
        let expected_instrument = Instrument::new(
            Exchange::Deribit, 
            market_type, 
            InstrumentType::Option{
                base: Currency::new(Cow::Borrowed("BTC")), 
                quote: Currency::new(Cow::Borrowed("USD")), 
                expiry: Cow::Borrowed("20250328"),
                strike: 100000, 
                kind: OptionKind::Call});
        let result = exchange.normalize(MarketType::OrderBook,instrument_name);
        println!("{:?}", result);
        assert_eq!(result, Some(expected_instrument));
    }

    #[test]
    fn test_normalize_perpetual1() {
        let instrument_name = "BTC-PERPETUAL";
        let exchange = DeribitHandler;
        let market_type = MarketType::OrderBook;
        let expected_instrument = Instrument::new(
            Exchange::Deribit, 
            market_type, 
            InstrumentType::Perpetual{
                base: Currency::new(Cow::Borrowed("BTC")), 
                quote: Currency::new(Cow::Borrowed("USD")), 
            });
        let result = exchange.normalize(MarketType::OrderBook,instrument_name);
        println!("{:?}", result);
        assert_eq!(result, Some(expected_instrument));
    }

    #[test]
    fn test_normalize_perpetual2() {
        let instrument_name = "SOL_USDC-PERPETUAL";
        let exchange = DeribitHandler;
        let market_type = MarketType::OrderBook;
        let expected_instrument = Instrument::new(
            Exchange::Deribit, 
            market_type, 
            InstrumentType::Perpetual{
                base: Currency::new(Cow::Borrowed("SOL")), 
                quote: Currency::new(Cow::Borrowed("USDC")), 
            });
        let result = exchange.normalize(MarketType::OrderBook,instrument_name);
        println!("{:?}", result);
        assert_eq!(result, Some(expected_instrument));
    }

    #[test]
    fn test_normalize_spot() {
        let instrument_name = "BTC_USD";
        let exchange = DeribitHandler;
        let market_type = MarketType::OrderBook;
        let expected_instrument = Instrument::new(
            Exchange::Deribit, 
            market_type, 
            InstrumentType::Spot{
                base: Currency::new(Cow::Borrowed("BTC")), 
                quote: Currency::new(Cow::Borrowed("USD")), 
            });
        let result = exchange.normalize(MarketType::OrderBook,instrument_name);
        println!("{:?}", result);
        assert_eq!(result, Some(expected_instrument));
    }

    #[test]
    fn test_normalize_unknown() {
        let instrument_name = "BTC-USD-20250528";
        let exchange = DeribitHandler;
        assert_eq!(exchange.normalize(MarketType::OrderBook, instrument_name), None);
    }
}

#[cfg(test)]
mod deribit_denormalize_tests{
    use std::borrow::Cow;

    use crate::{exchange::deribit::DeribitHandler, Currency, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType, OptionKind};

    #[test]
    fn test_denorm_future() {
        let instrument = Instrument::new(Exchange::Deribit, MarketType::OrderBook, InstrumentType::Future{
            base: Currency::new(Cow::Borrowed("BTC")), 
            quote: Currency::new(Cow::Borrowed("USD")),
            expiry: Cow::Borrowed("20250328")});
        let exchange = DeribitHandler;
        assert_eq!(exchange.denormalize(&instrument), Some(String::from("BTC-28MAR25")));
    }

    #[test]
    fn test_denorm_option() {
        let instrument = Instrument::new(
            Exchange::Deribit, 
            MarketType::OrderBook, 
            InstrumentType::Option{
                base: Currency::new(Cow::Borrowed("BTC")), 
                quote: Currency::new(Cow::Borrowed("USD")), 
                expiry: Cow::Borrowed("20250328"), 
                strike: 100000, 
                kind: OptionKind::Call});
        let exchange = DeribitHandler;
        assert_eq!(exchange.denormalize(&instrument), Some(String::from("BTC-28MAR25-100000-C")));
    }

    #[test]
    fn test_denorm_perp1() {
        let instrument = Instrument::new(
            Exchange::Deribit, 
            MarketType::OrderBook, 
            InstrumentType::Perpetual{
                base: Currency::new(Cow::Borrowed("BTC")), 
                quote: Currency::new(Cow::Borrowed("USD")), 
            });
        let exchange = DeribitHandler;
        assert_eq!(exchange.denormalize(&instrument), Some(String::from("BTC-PERPETUAL")));
    }

    #[test]
    fn test_denorm_perp2() {
        let instrument = Instrument::new(Exchange::Deribit, MarketType::OrderBook, InstrumentType::Perpetual{
            base: Currency::new(Cow::Borrowed("SOL")), 
            quote: Currency::new(Cow::Borrowed("USDC")), });
        let exchange = DeribitHandler;
        assert_eq!(exchange.denormalize(&instrument), Some(String::from("SOL_USDC-PERPETUAL")));
    }
    #[test]
    fn test_denorm_spot() {
        let instrument = Instrument::new(
            Exchange::Deribit, 
            MarketType::OrderBook, 
            InstrumentType::Spot{
                base: Currency::new(Cow::Borrowed("BTC")), 
                quote: Currency::new(Cow::Borrowed("USD")), 
            });
        let exchange = DeribitHandler;
        assert_eq!(exchange.denormalize(&instrument), Some(String::from("BTC_USD")));
    }
}