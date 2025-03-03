use tracing::error;

use crate::{Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType};

pub struct DydxHandler(pub Exchange);

impl ExchangeHandler for DydxHandler {

    fn normalize(&self, market_type: MarketType, instrument_name: String) -> Option<Instrument> {
        if self.0 != Exchange::Dydx {
            // The exchange is not Dydx; normalization is not supported
            error!("normalize::Expected {:?} got {:?}", Exchange::Dydx, self.0);
            return None;
        }

        if !self.market_type_validator(&market_type) {
            error!("denormalize::Market Type for {:?} is unsupported: {:?}", self.0, market_type);
            return None;
        }
        let parts: Vec<&str> = instrument_name.split('-').collect();
    
        match parts.as_slice() {
            [base, quote] if !["perp", "perpetual"].contains(&quote.to_lowercase().as_str()) => 
                Some(Instrument::new(
                    self.0, 
                    market_type, 
                    InstrumentType::Perpetual(base.to_string(), quote.to_string()),
                )),
            _ => {
                error!("normalize::Unexpected instrument format: {:?}", instrument_name);
                None
            }
        }
    }

    fn denormalize(&self, instrument: Instrument) -> Option<String> {
        if self.0 != Exchange::Dydx {
            error!("denormalize::Expected {:?} got {:?}", Exchange::Dydx, self.0);
            return None;
        }
        if !self.instrument_type_validator(&instrument.instrument_type) {
            error!("denormalize::Instrument Type for {:?} is unsupported: {:?}", self.0, instrument.instrument_type);
            return None;
        }

        if !self.market_type_validator(&instrument.market_type) {
            error!("denormalize::Market Type for {:?} is unsupported: {:?}", self.0, instrument.market_type);
            return None;
        }
        match instrument.instrument_type {
            InstrumentType::Perpetual(base, quote) => Some(format!("{}-{}", base, quote)),
            _ => None
        }
    }

    fn market_type_validator(&self, market_type: &MarketType) -> bool {
        match market_type {
            MarketType::OrderBook  => true,
            _ => false
        }
    }

    fn instrument_type_validator(&self, instrument_type: &InstrumentType) -> bool {
        match instrument_type {
            InstrumentType::Perpetual(_, _) => true,
            _ => false
        }
    }
}

#[cfg(test)]
mod dydx_normalize_tests{
    use crate::{exchange::dydx::DydxHandler, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType};

    #[test]
    fn test_normalize_perpetual() {
        let instrument_name = "BTC-USD".to_string();
        let exchange = DydxHandler(Exchange::Dydx);
        let market_type = MarketType::OrderBook;
        let expected_instrument = Instrument::new(Exchange::Dydx, market_type, InstrumentType::Perpetual("BTC".to_string(), "USD".to_string()));
        assert_eq!(exchange.normalize(MarketType::OrderBook, instrument_name), Some(expected_instrument));
    }
    #[test]
    fn test_normalize_unknown() {
        let instrument_name = "BTC-PERP".to_string();
        let exchange = DydxHandler(Exchange::Derive);
        assert_eq!(exchange.normalize(MarketType::OrderBook, instrument_name), None);
    }
}

#[cfg(test)]
mod dydx_denormalize_tests{
    use crate::{exchange::dydx::DydxHandler, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType};

    #[test]
    fn test_denorm_perp() {
        let instrument = Instrument::new(Exchange::Dydx, MarketType::OrderBook, InstrumentType::Perpetual("BTC".to_string(), "USD".to_string()));
        let exchange = DydxHandler(Exchange::Dydx);
        assert_eq!(exchange.denormalize(instrument), Some(String::from("BTC-USD")));
    }
}