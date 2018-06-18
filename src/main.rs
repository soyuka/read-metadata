// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

extern crate clap;
extern crate env_logger;
extern crate scraper;
extern crate reqwest;
extern crate url;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;

// @TODO: copied from error_chain without knowing what it does, investigate
mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain!{}
}

error_chain! {
    foreign_links {
        ReqError(reqwest::Error);
        IoError(std::io::Error);
    }
}

#[derive(Deserialize)]
struct Input {
    id: String,
    url: String
}

#[derive(Serialize)]
struct Output {
    id: String,
    title: String,
    description: String,
    og_image: String,
    favicon: Vec<u8>
}

use reqwest::header::ContentLength;
use clap::{App};
use scraper::{Html,Selector};
use std::io::prelude::*;
use std::io;
use url::{Url, ParseError};

/// Read a <meta> element's "content" attribute value
///
/// # Example
///
/// ```
/// read_meta_element(r#"meta[name="description"]"#, &Html::parse_document(&body))
/// ```
fn read_meta_element(selector: &str, document: &Html) -> String {
    let element = document.select(&Selector::parse(selector).unwrap()).next();
    let mut value = "".to_string();

    if element != None {
        value = element.unwrap().value().attr("content").unwrap().trim().to_string();
    }

    return value;
}

/// Get the favicon buffer if it's size is lower than 4Mb
///
/// # Example
///
/// ```
/// get_favicon(&"http://example.com/favicon.ico");
/// ```
fn get_favicon(favicon_url: String) -> Result<Vec<u8>> {
    let mut resp = reqwest::get(&favicon_url)?;
    let mut buffer: Vec<u8> = vec![];

    if resp.status().is_success() {
        let len = resp.headers().get::<ContentLength>()
                    .map(|ct_len| **ct_len)
                    .unwrap_or(0);

        // limit 4mb response
        if len <= 4_000_000 {
            buffer = Vec::with_capacity(len as usize);
            let _bytes = resp.copy_to(&mut buffer);
        }
    }

    Ok(buffer)
}

fn sanitize_href(href: String, origin: String) -> Result<String> {
    let mut parsed_href = Url::parse(&href);

    if parsed_href == Err(ParseError::RelativeUrlWithoutBase) {
        // operator ? should work here but looks like reqwest is overriding the types of Url?
        let parsed_origin = Url::parse(&origin).unwrap();
        parsed_href = parsed_origin.join(&href);
    }

    Ok(parsed_href.unwrap().as_str().to_string())
}

/// Reads metadata from a given url
///
/// # Example:
/// ```
/// read_metadata("http://soyuka.me", "uuid")
/// ```
fn read_metadata(url: String, id: String) -> Result<Output> {
    let body = reqwest::get(&url)?.text()?;
    let document = Html::parse_document(&body);

    let title = document.select(&Selector::parse("title").unwrap()).next();
    let mut title_value: String = "".to_string();

    if title != None {
        title_value = title.unwrap().text().map(|x| x.trim()).collect();
    }

    let mut favicon_buffer: Vec<u8> = vec![];
    // https://html.spec.whatwg.org/multipage/links.html#linkTypes
    let mut favicon = document.select(&Selector::parse(r#"link[rel="shortcut icon"]"#).unwrap()).next();

    if favicon == None {
        favicon = document.select(&Selector::parse(r#"link[rel="icon"]"#).unwrap()).next();
    }

    if favicon != None {
        let favicon_href = favicon.unwrap().value().attr("href").unwrap().to_string();
        favicon_buffer = get_favicon(sanitize_href(favicon_href, url)?).unwrap();
    }

    Ok(Output {
        id: id,
        title: title_value,
        description: read_meta_element(r#"meta[name="description"]"#, &document),
        og_image: read_meta_element(r#"meta[property="og:image"]"#, &document),
        favicon: favicon_buffer
    })
}

fn run () -> Result<()> {
    env_logger::init();
    App::new("read-metadata")
        .version("0.0.0")
        .author("soyuka <soyuka@gmail.com>")
        .about(
r#"Read website metadata, stdin should be json:

`{"id": "uuid", "url": "http://soyuka.me"}`
"#
)
        .get_matches();

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_n) => {
            let v: Input = serde_json::from_str(&input).unwrap();
            info!("Reading metadata for {}", v.url);
            let o: Output = read_metadata(v.url, v.id).unwrap();
            io::stdout().write(&serde_json::to_vec(&o).unwrap())?;
        }
        Err(error) => println!("error: {}", error),
    }

    Ok(())
}

quick_main!(run);
