use normify::{Exchange, ExchangeHandler, MarketType};
use normify::exchange::deribit::DeribitHandler;

fn main() {
    /*Deribit OPT*/
    let instrument_name_from_exchange = "BTC-28MAR25-100000-C";
    let market_type = MarketType::OrderBook;
    let exchange = DeribitHandler(Exchange::Deribit);
    let normalized_instrument = exchange.normalize(market_type, instrument_name_from_exchange.to_string());

    match normalized_instrument {
        Some(n) => {

            /* Print normalized  instrument */
            println!("Normalized Instrument: {:?}", n);

            /* Print standard instrument name */
            println!("Standard Format: {}", n.to_string());

            /* Denormalized  instrument */
            let instrument_name_from_exchange = exchange.denormalize(n).unwrap();

            /* Print exchange spicific  instrument  name */
            println!("Denormalized Instrument: {}", instrument_name_from_exchange);
        },
        None => {
            println!("Failed to normalize the instrument");
        }
    }
}