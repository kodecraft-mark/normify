use normify::{parse_standard_format, to_exchange_format, Exchange, MarketType};

fn main() {
    /*Deribit OPT*/
    // Case when you only have the exchange name and the Instrument Name
    let instrument_name_from_exchange = "BTC-28MAR25-100000-C";
    let exchange = "aevo";
    let exchange = match Exchange::try_from(exchange) {
        Ok(ex) => ex,
        Err(err) => {
            eprintln!("{err}");
            return;
        }
    };
    let ins = exchange.handler().normalize(MarketType::OrderBook, instrument_name_from_exchange).unwrap();
    println!("{:#?}", ins);

    // Case when you have the standard format
    let standard_format = "o.o.BTC-USDC-20250328-90000-C.aevo";
    let ins = match parse_standard_format(standard_format) {
        Ok(ins) => ins,
        Err(err) => {
            eprintln!("{err}");
            return;
        }
    };
    println!("{:#?}", ins);

    //Case when you want to get the exchange specific format
    let standard_format = "o.o.BTC-USD-20250328-90000-C.aevo";
    let ins = to_exchange_format(standard_format).unwrap();
    println!("Exchange specific format: {:#?}", ins);
    
}