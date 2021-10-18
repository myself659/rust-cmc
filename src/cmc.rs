use std::{fmt};
use std::{collections::HashMap};

use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CMCResponse {
    pub data: HashMap<String, Currency>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Currency {
    pub name: String,
    pub symbol: String,
    pub quote: Quotes,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Quotes(pub HashMap<String, Quote>);
// struct Quotes {
//     HashMap<String, Quote>
// }

#[derive(Debug, Serialize, Deserialize)]
pub struct Quote {
    pub price: f64,
    pub percent_change_7d: f64,
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Name: {}, Symbol: {} Price: {} change(7d): {}%",
            self.name,
            self.symbol,
            self.quote.0.get("USD").unwrap().price.to_string(),
            self.quote
                .0
                .get("USD")
                .unwrap()
                .percent_change_7d
                .to_string()
        )
    }
}

impl CMCResponse {
    fn get_currency(&self, currency: &str) -> Option<&Currency> {
        self.data.get(currency)
    }
}