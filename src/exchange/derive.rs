use tracing::error;

use crate::{denormalize_expiry, normalize_expiry, parse_expiry_date, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType, OptionKind};

const DEFAULT_QUOTE_CURRENCY: &str = "USD";
const DEFAULT_EXPIRY_FORMAT: &str = "%Y%m%d";
pub struct DeriveHandler(Exchange);

impl ExchangeHandler for DeriveHandler {

    fn normalize(&self, market_type: MarketType, instrument_name: String) -> Option<Instrument> {
        if self.0 != Exchange::Derive {
            // The exchange is not Derive; normalization is not supported
            error!("normalize::Expected {:?} got {:?}", Exchange::Derive, self.0);
            return None;
        }
    
        // Split the instrument name into parts
        let exchange = self.0;
        let parts: Vec<&str> = instrument_name.split('-').collect();
    
        match parts.as_slice() {
            // Perpetual: e.g., BTC-PERP
            [base, "perp" | "PERP"] => {
                Some(Instrument::new(exchange, market_type, InstrumentType::Perpetual(base.to_string(), DEFAULT_QUOTE_CURRENCY.to_string())))
            }
    
            // Option: e.g., BTC-20250328-100000-C
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
            // No matching format
            _ => {
                error!("normalize::Unexpected instrument format: {:?}", instrument_name);
                None
            }
        }
    }

    fn denormalize(&self, instrument: Instrument) -> Option<String> {
        if self.0 != Exchange::Derive {
            error!("denormalize::Expected {:?} got {:?}", Exchange::Derive, self.0);
            return None;
        }

        if !Self::instrument_type_validator(&instrument.instrument_type) {
            error!("denormalize::Instrument Type for {:?} is unsupported: {:?}", self.0, instrument.instrument_type);
            return None;
        }
        match instrument.instrument_type {
            InstrumentType::Option(base, _quote, expiry, strike, kind) => {
                let denormalize_expiry = denormalize_expiry(&expiry, DEFAULT_EXPIRY_FORMAT);
                Some(format!("{}-{}-{}-{}", base, denormalize_expiry, strike, kind.to_string()))
            },
            InstrumentType::Perpetual(base, _) => Some(format!("{}-PERP", base)),
            _ => None
        }
    }

    fn instrument_type_validator(instrument_type: &InstrumentType) -> bool {
        match instrument_type {
            InstrumentType::Option(_, _, _, _, _) => true,
            InstrumentType::Perpetual(_, _) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod derive_normalize_tests{
    use crate::{exchange::derive::DeriveHandler, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType};
    #[test]
    fn test_normalize_option() {
        let instrument_name = "BTC-20250328-100000-C".to_string();
        let exchange = DeriveHandler(Exchange::Derive);
        let market_type = MarketType::OrderBook;
        let expected_instrument = Instrument::new(Exchange::Derive, market_type, InstrumentType::Option("BTC".to_string(), "USD".to_string(), "20250328".to_string(), 100000, crate::OptionKind::Call));
        let result = exchange.normalize(MarketType::OrderBook, instrument_name);
        println!("{:?}", result);
        assert_eq!(result, Some(expected_instrument));
    }

    #[test]
    fn test_normalize_perpetual() {
        let instrument_name = "BTC-PERP".to_string();
        let exchange = DeriveHandler(Exchange::Derive);
        let market_type = MarketType::OrderBook;
        let expected_instrument = Instrument::new(Exchange::Derive, market_type, InstrumentType::Perpetual("BTC".to_string(), "USD".to_string()));
        assert_eq!(exchange.normalize(MarketType::OrderBook, instrument_name), Some(expected_instrument));
    }

    #[test]
    fn test_normalize_unknown() {
        let instrument_name = "BTC-28MAR25-100000-C".to_string();
        let exchange = DeriveHandler(Exchange::Derive);
        assert_eq!(exchange.normalize(MarketType::OrderBook, instrument_name), None);
    }
}

#[cfg(test)]
mod derive_denormalize_tests{
    use crate::{exchange::derive::DeriveHandler, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType};

    #[test]
    fn test_denorm_option() {
        let instrument = Instrument::new(Exchange::Derive, MarketType::OrderBook, InstrumentType::Option("BTC".to_string(), "USD".to_string(), "20250328".to_string(), 100000, crate::OptionKind::Call));
        let exchange = DeriveHandler(Exchange::Derive);
        assert_eq!(exchange.denormalize(instrument), Some(String::from("BTC-20250328-100000-C")));
    }

    #[test]
    fn test_denorm_perp() {
        let instrument = Instrument::new(Exchange::Derive, MarketType::OrderBook, InstrumentType::Perpetual("BTC".to_string(), "USD".to_string()));
        let exchange = DeriveHandler(Exchange::Derive);
        assert_eq!(exchange.denormalize(instrument), Some(String::from("BTC-PERP")));
    }
}