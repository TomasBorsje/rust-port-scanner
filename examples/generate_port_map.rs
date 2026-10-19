use html_cleaning::{presets, Document, HtmlCleaner};
use itertools::Itertools;
use regex::regex;
use reqwest::Client;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Write;

#[derive(Debug)]
struct PortDescription {
    description: String
}

const WIKIPEDIA_PORTS_URL: &str = "https://en.wikipedia.org/wiki/List_of_TCP_and_UDP_port_numbers";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>{

    let client = Client::builder()
        .user_agent("SimplePortScraper/0.1 (https://github.com/TomasBorsje)")
        .build()?;

    println!("Scraping https://en.wikipedia.org/wiki/List_of_TCP_and_UDP_port_numbers - please wait...");
    let body = client.get(WIKIPEDIA_PORTS_URL)
        .send()
        .await?
        .text()
        .await?;

    println!("Parsing scraped page...");
    let well_known_ports = table_extract::Table::find_by_id(&body, "mwVQ").expect("Couldn't find well known ports table");
    let registered_ports = table_extract::Table::find_by_id(&body, "mwDV0").expect("Couldn't find registered ports table");

    let mut port_map: HashMap<u16, Vec<PortDescription>> = HashMap::new();
    let cleaner = HtmlCleaner::with_options(presets::minimal());

    for table in [well_known_ports, registered_ports] {
        for row in table.iter() {
            let cells = row.as_slice();
            let description = clean_html(cells[cells.len()-1].to_owned(), &cleaner);
            let port_text = clean_html(cells[0].to_owned(), &cleaner);

            // If it is a port range
            if port_text.contains("–") {
                let port_ranges: Vec<&str> = port_text.split("–").collect();
                let port_min: u16 = port_ranges[0].parse().expect("Couldn't parse port range start");
                let port_max: u16 = port_ranges[1].parse().expect("Couldn't parse port range end");

                for port in port_min..port_max+1 {
                    if !port_map.contains_key(&port) {
                        port_map.insert(port, vec![]);
                    }
                    port_map.get_mut(&port).unwrap().push(PortDescription {
                        description: description.clone()
                    });
                }
            } else {
                // Else it's a single port
                match port_text.parse::<u16>() {
                    Ok(port) => {
                        if !port_map.contains_key(&port) {
                            port_map.insert(port, vec![]);
                        }
                        port_map.get_mut(&port).unwrap().push(PortDescription {
                            description: description.clone()
                        });
                    },
                    Err(_) => continue,
                };
            }
        }
    }

    // Write port function file
    println!("Writing ports to src/port_map.rs...");
    let mut file = File::create("src/port_map.rs").expect("Couldn't create port_map.rs");
    let header = r#"pub fn get_port_description(port: u16) -> String {
    match port {"#;
    writeln!(file, "{}", header).expect("Couldn't write port_map.rs header");

    for (port, vec) in port_map.iter().sorted_by_key(|k| k.0) {
        // Map vec into a flat string
        let descriptions = vec
            .iter()
            .map(|e| e.description.clone())
            .reduce(|acc, e| {
                let mut new_string = String::from(acc);
                new_string.push_str(" OR ");
                new_string.push_str(&e);
                new_string
            });
        writeln!(file, "        {0} => String::from(r#\"{1}\"#),", port, descriptions.unwrap_or(String::new())).expect("Couldn't write port line");
    }

    let footer = r#"        _ => String::from(""),
    }
}"#;
    writeln!(file, "{}", footer).expect("Couldn't write port_map.rs footer");
    println!("Successfully wrote ports to src/port_map.rs.");

    Ok(())
}

fn clean_html(string: String, cleaner: &HtmlCleaner) -> String {
    let regex = regex!(r"\[\d+]");
    let doc = Document::from(string);
    cleaner.clean(&doc);
    let doc_text: String = doc.text().into();
    regex.replace_all(&doc_text, "").trim().into()
}