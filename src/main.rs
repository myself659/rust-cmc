use std::{collections::HashMap, fmt::Display};

use serde::{Serialize, Deserialize};
use clap::{Arg, App};

#[derive(Debug, Serialize, Deserialize)]
struct CMCResponse {
    data: HashMap<String, Currency>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Currency {
    name: String,
    symbol: String,
    quote: Quotes,
}

#[derive(Debug, Serialize, Deserialize)]
struct Quotes(HashMap<String, Quote>);
// struct Quotes {
//     HashMap<String, Quote>
// }

#[derive(Debug, Serialize, Deserialize)]
struct Quote {
    price: f64,
    percent_change_7d: f64,
}

impl  Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Name: {}, Symbol: {} Price: {} change(7d): {}%",
        self.name,
        self.symbol,
        self.quote.0.get("USD").unwrap().price.to_string(),
        self.quote.0.get("USD").unwrap().percent_change_7d.to_string()
        )
    }
}

impl CMCResponse {
    fn get_currency(&self, currency: &str) -> Option<&Currency> {
        self.data.get(currency)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let  matches = App::new("RUST-CMC")
    .version("1.0")
    .about("rust cmc cmd")
    .arg(Arg::new("currency_list")
    .long("currencies")
    .short("c")
    .about("Pass the list of currencies you want to query")
    .min_values(1)
    .required(true)
).get_matches();

let currencies = matches.value_of("currency_list").expect("No currencies were being passed");

    let cmc_pro_api_key = dotenv::var("CMC_PRO_API_KEY").expect("CMC key not set");
    let mut params = HashMap::new();
    // params.insert("symbol", "BTC");
    params.insert("symbol", currencies.to_string());


    let client = reqwest::Client::new();
    let resp = client
        .get("https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest")
        .header("X-CMC_PRO_API_KEY", cmc_pro_api_key)
        .query(&params)
        .send()
        .await?;

    let prices  = resp.json::<CMCResponse>().await?;
    if let Some(bitcoin) = prices.get_currency("BTC"){
        println!("{}", bitcoin);
    }else {
        println!("bitcoin is not in the list");
    }

    Ok(())
}
