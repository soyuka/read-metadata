#![allow(warnings)]

extern crate clap;
extern crate scraper;
extern crate reqwest;
extern crate env_logger;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_json;

use clap::{Arg, App};
use scraper::{Html,Selector};

error_chain! {
    foreign_links {
        ReqError(reqwest::Error);
        IoError(std::io::Error);
    }
}

// todo: https://github.com/sozu-proxy/lapin
fn run() -> Result<()> {

    let matches = App::new("read-metadata")
        .version("0.0.0")
        .author("soyuka <soyuka@gmail.com>")
        .about("Read website metadata")
        .arg(Arg::with_name("URL").required(true).takes_value(true).index(1).help("URL to read metadata from"))
        .get_matches();

    let url = matches.value_of("URL").unwrap();
    // println!("Fetching metadata for url: {}", url);

    let body = reqwest::get(url)?.text()?;
    let document = Html::parse_document(&body);

    let title = document.select(&Selector::parse("title").unwrap()).next();
    let mut titleValue = "".to_string();

    if (title != None) {
        titleValue = title.unwrap().text().map(String::from).collect();
    }

    let description = document.select(&Selector::parse(r#"meta[name="description"]"#).unwrap()).next();
    let mut descriptionValue = "".to_string();

    if (description != None) {
        descriptionValue = description.unwrap().value().attr("content").unwrap().to_string();
    }

    let favicon = document.select(&Selector::parse(r#"link[rel="icon"]"#).unwrap()).next();
    let mut faviconBuf: Vec<u8> = vec![];

    if (favicon != None) {
        let faviconUrl = favicon.unwrap().value().attr("href").unwrap();
        let mut res = reqwest::get(faviconUrl)?;
        res.copy_to(&mut faviconBuf);
        // println!("favicon = {:?}", faviconUrl);
    }

    let data = json!({
        "title": titleValue,
        "description": descriptionValue,
        "favicon": faviconBuf
    });

    println!("{}", data.to_string());

    Ok(())
}

quick_main!(run);
