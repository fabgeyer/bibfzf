#[macro_use]
extern crate lazy_static;
extern crate clap;
extern crate nom_bibtex;
extern crate prettytable;
extern crate regex;
extern crate skim;

use clap::{App, Arg};
use nom_bibtex::Bibtex;
use prettytable::{cell, format, row, Table};
use regex::Regex;
use skim::{Skim, SkimOptions};
use std::default::Default;
use std::fs::File;
use std::io::prelude::*;
use std::io::Cursor;
use std::process;

fn read_file(filename: &str) -> String {
    let mut file = File::open(filename).unwrap();
    let mut bib_content = String::new();

    file.read_to_string(&mut bib_content).unwrap();
    bib_content
}

fn strformat(val: &str) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"\s+").unwrap();
    }
    RE.replace_all(val, " ").to_string()
}

fn main() {
    let app = App::new("bibfzf")
        .version("18.11")
        .arg(
            Arg::with_name("key")
                .short("k")
                .long("key")
                .help("Key to display from bibtex file")
                .takes_value(true),
        ).arg(
            Arg::with_name("bibtex")
                .help("Input bibtex file")
                .required(true)
                .value_name("BIBTEX"),
        );

    let matches = app.get_matches();
    let bib_path = matches.value_of("bibtex").unwrap();
    let bib_str = read_file(bib_path);
    let bibtex = Bibtex::parse(&bib_str).unwrap();

    if let Some(bibkey) = matches.value_of("key") {
        let mut table = Table::new();
        let format = format::FormatBuilder::new().column_separator(' ').build();
        table.set_format(format);

        for entry in bibtex.bibliographies() {
            if entry.citation_key() != bibkey {
                continue;
            }

            table.add_row(row![b->"key", bibkey]);
            table.add_row(row![b->"type", entry.entry_type()]);

            for tag in entry.tags() {
                let (k, v) = tag;
                table.add_row(row![b->k, strformat(v)]);
            }
            table.printstd();
            process::exit(0);
        }
        println!("Couldn't find key {}", bibkey);
        process::exit(1);
    }

    let mut v: Vec<String> = Vec::new();
    'outer: for entry in bibtex.bibliographies() {
        let mut title: String = "".to_string();
        let mut author: String = "".to_string();
        let mut year: String = "".to_string();
        for tag in entry.tags() {
            let (k, v) = tag;
            let lk = k.to_ascii_lowercase();
            match lk.as_ref() {
                "title" => {
                    title = strformat(&v).to_string();
                }
                "author" => {
                    author = strformat(&v).to_string();
                }
                "year" => {
                    year = strformat(&v).to_string();
                }
                _ => {}
            }
        }
        v.push(format!(
            "{} | {}",
            entry.citation_key(),
            [title, author, year].join(" - ")
        ));
    }

    // Display list of keys using skim
    let preview = format!(
        "{} --key {{1}} {}",
        std::env::current_exe().unwrap().to_str().unwrap(),
        bib_path
    );
    let options: SkimOptions = SkimOptions::default()
        .multi(false)
        .delimiter("|")
        .preview(preview.as_str());
    let selected_items = Skim::run_with(&options, Some(Box::new(Cursor::new(v.join("\n")))))
        .map(|out| out.selected_items)
        .unwrap_or_else(|| Vec::new());

    for item in selected_items.iter() {
        println!("{}", item.get_output_text());
    }
}
