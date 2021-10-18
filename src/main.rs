use std::{collections::HashMap, fmt};

use clap::{App, Arg};
// use csv::Writer;
use log::{debug, error, info};
use log4rs;
use serde::{Deserialize, Serialize};

extern crate google_sheets4 as sheets4;
extern crate yup_oauth2 as oauth2;
use sheets4::api::ValueRange;
use sheets4::Error;
use sheets4::Sheets;
use yup_oauth2::read_service_account_key;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref SHEET_ID: &'static str = "1kGe3O8h7quZpROV7ct0bSgICZRQmZWhppEttF2h9M8Y";
    static ref SECRET_PATH: &'static str = "secret.json";
}

#[derive(Serialize, Deserialize, Debug)]
struct EODResponse {
    code: String,
    close: f64,
}

#[derive(Debug)]
enum OneError {
    NoAPIKey,
    CSV(csv::Error),
    IO(std::io::Error),
    Reqwest(reqwest::Error),
}

impl std::error::Error for OneError {}

impl fmt::Display for OneError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OneError::NoAPIKey => write!(f, "No API key is set via the .env variable."),
            OneError::CSV(err) => write!(f, "Error while writing the CSV file {}", err),
            OneError::IO(err) => write!(f, "Error while flushing the file {}", err),
            OneError::Reqwest(err) => write!(f, "Error while fetching data {}", err),
        }
    }
}

impl From<reqwest::Error> for OneError {
    fn from(err: reqwest::Error) -> OneError {
        OneError::Reqwest(err)
    }
}

impl From<csv::Error> for OneError {
    fn from(err: csv::Error) -> OneError {
        OneError::CSV(err)
    }
}

impl From<std::io::Error> for OneError {
    fn from(err: std::io::Error) -> OneError {
        OneError::IO(err)
    }
}

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

#[tokio::main]
async fn main() -> Result<(), OneError> {
    dotenv::dotenv().ok();
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    let matches = App::new("RUST-CMC")
        .version("1.0")
        .about("rust cmc cmd")
        .arg(
            Arg::new("currency_list")
                .long("currencies")
                .about("Pass the list of currencies you want to query")
                .min_values(1)
                .required(true),
        )
        .arg(
            Arg::new("etfs")
                .long("etfs")
                .about("Pass the ETF symbol to fetch price for")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let currencies = matches
        .value_of("currency_list")
        .expect("No currencies were being passed");
    let etfs = matches.value_of("etfs").expect("No ETF symbol passed");

    debug!("Querying the following currencies: {:?}", currencies);

    let eod_token = dotenv::var("EOD_TOKEN").expect("EOD token not set");
    let cmc_pro_api_key = dotenv::var("CMC_PRO_API_KEY").expect("CMC key not set");
    if cmc_pro_api_key.is_empty() {
        error!("Empty CMC API KEY provided! Please set one via the .env file!");
        return Err(OneError::NoAPIKey);
    }
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

    let prices = resp.json::<CMCResponse>().await?;
    /*
    let mut price_writer = Writer::from_path("prices.csv")?;
    price_writer.write_record(&["Name", "Symbol", "Price", "7DayChange"])?;
    for (symbol, currency) in prices.data.into_iter() {
        price_writer.write_record(&[
            currency.name,
            symbol.to_owned(),
            currency.quote.0.get("USD").unwrap().price.to_string(),
            currency
                .quote
                .0
                .get("USD")
                .unwrap()
                .percent_change_7d
                .to_string(),
        ])?;
    }
    price_writer.flush()?;
    */

    info!("Queried {} and wrote CSV file", currencies);

    let etf = client
        .get(format!(
            "https://eodhistoricaldata.com/api/real-time/{}?api_token={}&fmt=json",
            etfs, eod_token
        ))
        .send()
        .await?;
    let amundi_etf = etf.json::<EODResponse>().await?;
    debug!("Fetched ETF: {}", amundi_etf.close);
    let coins = ValueRange {
        major_dimension: Some("COLUMNS".to_string()),
        range: Some(format!("{}!{}2:{}4", "Crypto", "C", "C").to_owned()),
        values: Some(vec![vec![
            prices.data.get(&"BTC".to_owned()).unwrap().quote.0.get("USD").unwrap().price.to_string(),
            prices.data.get(&"ETH".to_owned()).unwrap().quote.0.get("USD").unwrap().price.to_string(),
            prices.data.get(&"DOGE".to_owned()).unwrap().quote.0.get("USD").unwrap().price.to_string(),
            ]]),
    };

    update_google_sheet(&SECRET_PATH, coins).await;

    Ok(())
}

async fn update_google_sheet(secret_path: &str, values: ValueRange) {
    let authenticator = yup_oauth2::ServiceAccountAuthenticator::builder(
        read_service_account_key(secret_path).await.unwrap(),
    )
    .build()
    .await
    .expect("Failed to create authenticator");

    let hub = Sheets::new(
        hyper::Client::builder().build(hyper_rustls::HttpsConnector::with_native_roots()),
        authenticator,
    );
    let range = values.clone().range.unwrap();
    let result = hub
        .spreadsheets()
        .values_update(values.clone(), &SHEET_ID, &values.range.unwrap())
        .value_input_option("USER_ENTERED")
        .doit()
        .await;

    match result {
        Err(e) => match e {
            Error::HttpError(_)
            | Error::Io(_)
            | Error::MissingAPIKey
            | Error::MissingToken(_)
            | Error::Cancelled
            | Error::UploadSizeLimitExceeded(_, _)
            | Error::Failure(_)
            | Error::BadRequest(_)
            | Error::FieldClash(_)
            | Error::JsonDecodeError(_, _) => {
                eprintln!("{}", e)
            }
        },
        Ok((_, _)) => info!("{} Updated range: {}", chrono::offset::Utc::now(), range),
    }
}
