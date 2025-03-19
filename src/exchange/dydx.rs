use std::borrow::Cow;

use tracing::error;

use crate::{Currency, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType};

const LOG_CTX: &str = "normify::exchange#dydx";
pub struct DydxHandler;
// Create a static instance to avoid allocations
pub static DYDX_HANDLER: DydxHandler = DydxHandler;

impl ExchangeHandler for DydxHandler {

    fn normalize(&self, market_type: MarketType, instrument_name: &str) -> Option<Instrument> {

        if !self.supports_market_type(&market_type) {
            error!(name: LOG_CTX, "denormalize::Market Type is unsupported: {:?}", market_type);
            return None;
        }
        let parts: Vec<&str> = instrument_name.split('-').collect();
    
        match parts.as_slice() {
            [base, quote] => {
                Some(Instrument::new(
                    Exchange::Dydx, 
                    market_type, 
                    InstrumentType::Perpetual {
                        base: Currency::new(Cow::Owned(base.to_string())), 
                        quote: Currency::new(Cow::Owned(quote.to_string())),
                    }
                ))
            },
            _ => {
                error!(name: LOG_CTX, "normalize::Unexpected instrument format: {:?}", instrument_name);
                None
            }
        }
    }

    fn denormalize(&self, instrument: &Instrument) -> Option<String> {
        if instrument.exchange != Exchange::Dydx {
            error!(name: LOG_CTX, "denormalize::Attempted to use Dydx handler for {:?}", instrument.exchange);
            return None;
        }
        if !self.supports_instrument_type(&instrument.instrument_type) {
            error!(name: LOG_CTX, "denormalize::Instrument Type for {:?} is unsupported", instrument.instrument_type);
            return None;
        }

        if !self.supports_market_type(&instrument.market_type) {
            error!(name: LOG_CTX, "denormalize::Market Type for {:?} is unsupported", instrument.market_type);
            return None;
        }
        match &instrument.instrument_type {
            InstrumentType::Perpetual{base, quote} => Some(format!("{}-{}", base.as_ref(), quote.as_ref())),
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
mod dydx_normalize_tests{
    use std::borrow::Cow;

    use crate::{exchange::dydx::DydxHandler, Currency, Exchange, ExchangeHandler, Instrument, InstrumentType, MarketType};

    #[test]
    fn test_normalize_perpetual() {
        let instrument_name = "BTC-USD";
        let exchange = DydxHandler;
        let market_type = MarketType::OrderBook;
        let expected_instrument = Instrument::new(
            Exchange::Dydx, 
            market_type, 
            InstrumentType::Perpetual{
            base: Currency::new(Cow::Borrowed("BTC")), 
            quote: Currency::new(Cow::Borrowed("USD"))
        });
        assert_eq!(exchange.normalize(MarketType::OrderBook, instrument_name), Some(expected_instrument));
    }
    #[test]
    fn test_normalize_unknown() {
        let instrument_name = "BTC-PERP".to_string();
        let exchange = DydxHandler;
        assert_eq!(exchange.normalize(MarketType::Ticker, &instrument_name), None);
    }
}

#[cfg(test)]
mod dydx_denormalize_tests{
    use std::borrow::Cow;

    use crate::{exchange::dydx::DydxHandler,ExchangeHandler, Currency, Exchange, Instrument, InstrumentType, MarketType};

    #[test]
    fn test_denorm_perp() {
        let instrument = Instrument::new(
            Exchange::Dydx, 
            MarketType::OrderBook,
            InstrumentType::Perpetual{
                base: Currency::new(Cow::Borrowed("btc")), 
                quote: Currency::new(Cow::Borrowed("USD"))
            });
        let exchange = DydxHandler;
        assert_eq!(exchange.denormalize(&instrument), Some(String::from("BTC-USD")));
    }
}