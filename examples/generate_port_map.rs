use html_cleaning::{presets, Document, HtmlCleaner};
use regex::regex;
use reqwest::Client;
use std::error::Error;
use std::fs::File;
use std::io::Write;

#[derive(Debug)]
struct PortEntry {
    port: u16,
    description: String
}

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

    let mut port_entries: Vec<PortEntry> = vec![];
    let cleaner = HtmlCleaner::with_options(presets::minimal());

    for table in [well_known_ports, registered_ports] {
        for row in table.iter() {
            let cells = row.as_slice();
            let description = clean_html(cells[cells.len()-1].to_owned(), &cleaner);
            let port_text = clean_html(cells[0].to_owned(), &cleaner);

            if port_text.contains("–") {
                let port_ranges: Vec<&str> = port_text.split("–").collect();
                let port_min: u16 = port_ranges[0].parse().expect("Couldn't parse port range start");
                let port_max: u16 = port_ranges[1].parse().expect("Couldn't parse port range end");

                for port in port_min..port_max+1 {
                    port_entries.push(PortEntry {
                        port,
                        description: description.clone(),
                    });
                }
            } else {
                match port_text.parse::<u16>() {
                    Ok(port) => {
                        port_entries.push(PortEntry {
                            port,
                            description: description.clone(),
                        });
                    },
                    Err(_) => continue,
                };
            }
        }
    }

    // Write port function file
    // TODO: Some port ranges overlap, need to merge them if they share a port
    let mut file = File::create("src/port_map.rs").expect("Couldn't create port_map.rs");
    let header = r#"pub fn get_port_description(port: u16) -> String {
    match port {"#;
    writeln!(file, "{}", header).expect("Couldn't write port_map.rs header");

    for entry in port_entries {
        writeln!(file, "        {0} => String::from(r#\"{1}\"#),", entry.port, entry.description).expect("Couldn't write port line");
    }

    let footer = r#"        _ => String::from(""),
    }
}"#;
    writeln!(file, "{}", footer).expect("Couldn't write port_map.rs footer");

    Ok(())
}

fn clean_html(string: String, cleaner: &HtmlCleaner) -> String {
    let regex = regex!(r"\[\d+]");
    let doc = Document::from(string);
    cleaner.clean(&doc);
    let doc_text: String = doc.text().into();
    regex.replace_all(&doc_text, "").trim().into()
}