use std::borrow::Cow;

use tracing::error;

use crate::{Currency, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType};

const LOG_CTX: &str = "normify::exchange#paradex";
pub struct ParadexHandler;
        
// Create a static instance to avoid allocations
pub static PARADEX_HANDLER: ParadexHandler = ParadexHandler;

impl ExchangeHandler for ParadexHandler {

    fn normalize(&self, market_type: MarketType, instrument_name: &str) -> Option<Instrument> {

        if !self.supports_market_type(&market_type) {
            error!(name: LOG_CTX, "denormalize::Market Type for is unsupported: {:?}", market_type);
            return None;
        }
        let parts: Vec<&str> = instrument_name.split('-').collect();
    
        match parts.as_slice() {
            [base, quote, "perp" | "PERP"] => 
                Some(Instrument::new(
                    Exchange::Paradex, 
                    market_type, 
                    InstrumentType::Perpetual {
                        base: Currency(Cow::Owned(base.to_string())), 
                        quote: Currency(Cow::Owned(quote.to_string())),
                    }
                )),
            _ => {
                error!(name: LOG_CTX, "normalize::Unexpected instrument format: {:?}", instrument_name);
                None
            }
        }
    }

    fn denormalize(&self, instrument: &Instrument) -> Option<String> {
        if instrument.exchange != Exchange::Paradex {
            error!(name: LOG_CTX, "denormalize::Attempted to use Paradex handler for {:?}", instrument.exchange);
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
            InstrumentType::Perpetual{base, quote} => Some(format!("{}-{}-PERP", base.as_ref(), quote.as_ref())),
            _ => None
        }
    }

    fn supports_market_type(&self, market_type: &MarketType) -> bool {
        matches!(market_type, MarketType::OrderBook)
    }

    fn supports_instrument_type(&self, instrument_type: &InstrumentType) -> bool {
        matches!(instrument_type, InstrumentType::Perpetual { base: _, quote: _ })
    }
}

#[cfg(test)]
mod paradex_normalize_tests{
    use std::borrow::Cow;

    use crate::{exchange::paradex::ParadexHandler, Currency, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType};

    #[test]
    fn test_normalize_perpetual() {
        let instrument_name = "BTC-USD-PERP".to_string();
        let exchange = ParadexHandler;
        let market_type = MarketType::OrderBook;
        let expected_instrument = Instrument::new(
            Exchange::Paradex, 
            market_type, 
            InstrumentType::Perpetual {
                base: Currency::new(Cow::Borrowed("BTC")), 
                quote: Currency::new(Cow::Borrowed("USD"))
            });
        assert_eq!(exchange.normalize(MarketType::OrderBook, &instrument_name), Some(expected_instrument));
    }
    #[test]
    fn test_normalize_unknown() {
        let instrument_name = "BTC-PERP".to_string();
        let exchange = ParadexHandler;
        assert_eq!(exchange.normalize(MarketType::OrderBook, &instrument_name), None);
    }
}

#[cfg(test)]
mod paradex_denormalize_tests{
    use std::borrow::Cow;

    use crate::{exchange::paradex::ParadexHandler, Currency, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType};

    #[test]
    fn test_denorm_perp() {
        let instrument = Instrument::new(
            Exchange::Paradex, 
            MarketType::OrderBook, 
            InstrumentType::Perpetual{
                base: Currency::new(Cow::Borrowed("BTC")), 
                quote: Currency::new(Cow::Borrowed("USD"))
            });
        let exchange = ParadexHandler;
        assert_eq!(exchange.denormalize(&instrument), Some(String::from("BTC-USD-PERP")));
    }
}