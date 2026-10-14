use std::error::Error;
use html_cleaning::{presets, Document, HtmlCleaner};
use reqwest::Client;

const WIKIPEDIA_PORTS_URL: &str = "https://en.wikipedia.org/wiki/List_of_TCP_and_UDP_port_numbers";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>{
    let client = Client::builder()
        .user_agent("SimplePortScraper/0.1 (https://github.com/TomasBorsje)")
        .build()?;

    let body = client.get(WIKIPEDIA_PORTS_URL)
        .send()
        .await?
        .text()
        .await?;

    let well_known_ports = table_extract::Table::find_by_id(&body, "mwVQ").expect("Couldn't find well known ports table");
    let registered_ports = table_extract::Table::find_by_id(&body, "mwDV0").expect("Couldn't find registered ports table");

    let cleaner = HtmlCleaner::with_options(presets::standard());

    for row in well_known_ports.iter() {
        let port = row.get("Port");
        let desc = row.get("Description").expect("Couldn't get desc");
        let doc = Document::from(desc);
        cleaner.clean(&doc);
        let clean_desc = doc.text();

        println!("{clean_desc}")
    }

    Ok(())
}