use tracing::error;

use crate::{denormalize_expiry, normalize_expiry, parse_expiry_date, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType, OptionKind};

const DEFAULT_QUOTE_CURRENCY: &str = "USD";
const DEFAULT_EXPIRY_FORMAT: &str = "%d%b%y";
pub struct DeribitHandler(pub Exchange);

impl ExchangeHandler for DeribitHandler {

    fn normalize(&self, market_type: MarketType, instrument_name: String) -> Option<Instrument> {
        if self.0 != Exchange::Deribit {
            // The exchange is not Deribit; normalization is not supported
            error!("normalize::Expected {:?} got {:?}", Exchange::Deribit, self.0);
            return None;
        }
    
        // Split the instrument name into parts
        let exchange = self.0;
        let parts: Vec<&str> = instrument_name.split('-').collect();
    
        match parts.as_slice() {
            // Perpetual: e.g., BTC-PERPETUAL or SOL_USDC-PERPETUAL (Non USD quote)
            [base_quote, "perpetual" | "PERPETUAL"] => {
                let (base, quote) = base_quote.split_once('_').unwrap_or((base_quote, DEFAULT_QUOTE_CURRENCY));
                Some(Instrument::new(exchange, market_type, InstrumentType::Perpetual(base.to_string(), quote.to_string())))
            }
    
            // Future: e.g., BTC-28MAR25
            [base, expiry] if parse_expiry_date(expiry, DEFAULT_EXPIRY_FORMAT).is_some() => {
                let normalized_expiry  = normalize_expiry(expiry)?;
                Some(Instrument::new(exchange, market_type, InstrumentType::Future(base.to_string(), DEFAULT_QUOTE_CURRENCY.to_string(), normalized_expiry)))
            }
    
            // Option: e.g., BTC-28MAR25-100000-C
            [base, expiry, strike_str, kind_str] => {
                let _ = parse_expiry_date(expiry, DEFAULT_EXPIRY_FORMAT)?;
                let strike = strike_str.parse::<u64>().ok()?;
                let kind = OptionKind::from_str(kind_str)?;
                let normalized_expiry  = normalize_expiry(expiry)?;
                Some(Instrument::new(
                    exchange,
                    market_type,
                    InstrumentType::Option(base.to_string(), DEFAULT_QUOTE_CURRENCY.to_string(), normalized_expiry, strike, kind),
                ))
            }
    
            // Spot: e.g., BTC_USD
            [spot] => {
                let (base, quote) = spot.split_once('_')?;
                Some(Instrument::new(exchange, market_type, InstrumentType::Spot(base.to_string(), quote.to_string())))
            }
    
            // No matching format
            _ => {
                error!("normalize::Unexpected instrument format: {:?}", instrument_name);
                None
            }
        }
    }

    fn denormalize(&self, instrument: Instrument) -> Option<String> {
        if self.0 != Exchange::Deribit {
            error!("denormalize::Expected {:?} got {:?}", Exchange::Deribit, self.0);
            return None;
        }
        match instrument.instrument_type {
            InstrumentType::Future(base, _quote, expiry) => {
                let denormalize_expiry = denormalize_expiry(&expiry, DEFAULT_EXPIRY_FORMAT);
                Some(format!("{}-{}", base, denormalize_expiry))
            }
            InstrumentType::Option(base, _quote, expiry, strike, kind) => {
                let denormalize_expiry = denormalize_expiry(&expiry, DEFAULT_EXPIRY_FORMAT);
                Some(format!("{}-{}-{}-{}", base, denormalize_expiry, strike, kind.to_string()))
            }
            InstrumentType::Spot(base, quote) => Some(format!("{}_{}", base, quote)),
            InstrumentType::Perpetual(base, quote) => {
                if quote == DEFAULT_QUOTE_CURRENCY {
                    Some(format!("{}-PERPETUAL", base))
                }else{
                    Some(format!("{}_{}-PERPETUAL", base, quote))
                }
            }
        }
    }
}

#[cfg(test)]
mod deribit_normalize_tests{
    use crate::{exchange::deribit::DeribitHandler, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType};

    #[test]
    fn test_normalize_future() {
        let instrument_name = "BTC-28MAR25".to_string();
        let exchange = DeribitHandler(Exchange::Deribit);
        let market_type = MarketType::OrderBook;
        let expected_instrument = Instrument::new(Exchange::Deribit, market_type, InstrumentType::Future("BTC".to_string(), "USD".to_string(), "20250328".to_string()));
        let result = exchange.normalize(MarketType::OrderBook,instrument_name);
        println!("{:?}", result);
        assert_eq!(result, Some(expected_instrument));
    }

    #[test]
    fn test_normalize_option() {
        let instrument_name = "BTC-28MAR25-100000-C".to_string();
        let exchange = DeribitHandler(Exchange::Deribit);
        let market_type = MarketType::OrderBook;
        let expected_instrument = Instrument::new(Exchange::Deribit, market_type, InstrumentType::Option("BTC".to_string(), "USD".to_string(), "20250328".to_string(), 100000, crate::OptionKind::Call));
        let result = exchange.normalize(MarketType::OrderBook, instrument_name);
        println!("{:?}", result);
        assert_eq!(result, Some(expected_instrument));
    }

    #[test]
    fn test_normalize_perpetual1() {
        let instrument_name = "BTC-PERPETUAL".to_string();
        let exchange = DeribitHandler(Exchange::Deribit);
        let market_type = MarketType::OrderBook;
        let expected_instrument = Instrument::new(Exchange::Deribit, market_type, InstrumentType::Perpetual("BTC".to_string(), "USD".to_string()));
        assert_eq!(exchange.normalize(MarketType::OrderBook, instrument_name), Some(expected_instrument));
    }

    #[test]
    fn test_normalize_perpetual2() {
        let instrument_name = "SOL_USDC-PERPETUAL".to_string();
        let exchange = DeribitHandler(Exchange::Deribit);
        let market_type = MarketType::OrderBook;
        let expected_instrument = Instrument::new(Exchange::Deribit, market_type, InstrumentType::Perpetual("SOL".to_string(), "USDC".to_string()));
        assert_eq!(exchange.normalize(MarketType::OrderBook, instrument_name), Some(expected_instrument));
    }

    #[test]
    fn test_normalize_spot() {
        let instrument_name = "BTC_USD".to_string();
        let exchange = DeribitHandler(Exchange::Deribit);
        let market_type = MarketType::OrderBook;
        let expected_instrument = Instrument::new(Exchange::Deribit, market_type, InstrumentType::Spot("BTC".to_string(), "USD".to_string()));
        assert_eq!(exchange.normalize(MarketType::OrderBook, instrument_name), Some(expected_instrument));
    }

    #[test]
    fn test_normalize_unknown() {
        let instrument_name = "BTC-USD-20250528".to_string();
        let exchange = DeribitHandler(Exchange::Deribit);
        assert_eq!(exchange.normalize(MarketType::OrderBook, instrument_name), None);
    }
}

#[cfg(test)]
mod deribit_denormalize_tests{
    use crate::{exchange::deribit::DeribitHandler, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType};

    #[test]
    fn test_denorm_future() {
        let instrument = Instrument::new(Exchange::Deribit, MarketType::OrderBook, InstrumentType::Future("BTC".to_string(), "USD".to_string(), "20250328".to_string()));
        let exchange = DeribitHandler(Exchange::Deribit);
        assert_eq!(exchange.denormalize(instrument), Some(String::from("BTC-28MAR25")));
    }

    #[test]
    fn test_denorm_option() {
        let instrument = Instrument::new(Exchange::Deribit, MarketType::OrderBook, InstrumentType::Option("BTC".to_string(), "USD".to_string(), "20250328".to_string(), 100000, crate::OptionKind::Call));
        let exchange = DeribitHandler(Exchange::Deribit);
        assert_eq!(exchange.denormalize(instrument), Some(String::from("BTC-28MAR25-100000-C")));
    }

    #[test]
    fn test_denorm_perp1() {
        let instrument = Instrument::new(Exchange::Deribit, MarketType::OrderBook, InstrumentType::Perpetual("BTC".to_string(), "USD".to_string()));
        let exchange = DeribitHandler(Exchange::Deribit);
        assert_eq!(exchange.denormalize(instrument), Some(String::from("BTC-PERPETUAL")));
    }

    #[test]
    fn test_denorm_perp2() {
        let instrument = Instrument::new(Exchange::Deribit, MarketType::OrderBook, InstrumentType::Perpetual("SOL".to_string(), "USDC".to_string()));
        let exchange = DeribitHandler(Exchange::Deribit);
        assert_eq!(exchange.denormalize(instrument), Some(String::from("SOL_USDC-PERPETUAL")));
    }
    #[test]
    fn test_denorm_spot() {
        let instrument = Instrument::new(Exchange::Deribit, MarketType::OrderBook, InstrumentType::Spot("BTC".to_string(), "USD".to_string()));
        let exchange = DeribitHandler(Exchange::Deribit);
        assert_eq!(exchange.denormalize(instrument), Some(String::from("BTC_USD")));
    }
}