#[macro_use]
extern crate lazy_static;
extern crate clap;
extern crate config;
extern crate glob;
extern crate nom_bibtex;
extern crate prettytable;
extern crate regex;
extern crate skim;
extern crate dirs;

use clap::{App, Arg};
use config::Config;
use glob::glob;
use nom_bibtex::{Bibtex, Bibliography};
use prettytable::{cell, format, row, Table};
use regex::Regex;
use skim::{Skim, SkimOptions};
use std::collections::HashMap;
use std::default::Default;
use std::fs::File;
use std::io::prelude::*;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process;
use std::process::Command;

struct PBibliography<'a> {
    bib: &'a Bibliography<'a>,
    fields: HashMap<String, &'a String>,
}

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

fn action_open_pdf(biblio: &PBibliography, settings: &Config) {
    if biblio.fields.contains_key("file") {
        let fields = biblio
            .fields
            .get("file")
            .unwrap()
            .split(":")
            .collect::<Vec<_>>();
        if fields.len() < 3 {
            println!("'file' field not recognized");
            return;
        }
        let _res = Command::new(settings.get_str("actions.open_pdf").unwrap())
            .arg(fields[1])
            .status();
        return;
    }
    println!("No PDF in entry {}", biblio.bib.citation_key());
}

fn action_open_doi(biblio: &PBibliography, settings: &Config) {
    if biblio.fields.contains_key("doi") {
        let _res = Command::new(settings.get_str("actions.open_doi").unwrap())
            .arg(format!(
                "https://dx.doi.org/{}",
                biblio.fields.get("doi").unwrap()
            ))
            .status();
        return;
    }
    println!("No DOI in entry {}", biblio.bib.citation_key());
}

fn action_open_url(biblio: &PBibliography, settings: &Config) {
    if biblio.fields.contains_key("url") {
        let _res = Command::new(settings.get_str("actions.open_url").unwrap())
            .arg(biblio.fields.get("url").unwrap())
            .status();
        return;
    }
    println!("No URL in entry {}", biblio.bib.citation_key());
}

fn action_copy_key(biblio: &PBibliography, settings: &Config) {
    println!("{}", biblio.bib.citation_key());
}

fn action_copy_cite(biblio: &PBibliography, settings: &Config) {
    println!("\\cite{{{}}}", biblio.bib.citation_key());
}

fn check_key(biblio: &PBibliography, key: &str) -> bool {
    if key.is_empty() {
        return true;
    }
    return biblio.fields.contains_key(key);
}

fn actions_menu(biblio: &PBibliography, settings: &Config) {
    let actions: Vec<(&str, &str, fn(&PBibliography, &Config))> =
        vec![
            ("Open PDF", "file", action_open_pdf),
            ("Open URL", "url", action_open_url),
            ("Open DOI", "doi", action_open_doi),
            ("Copy key", "", action_copy_key),
            ("Copy \\cite", "", action_copy_cite),
        ];
    let actionsf = actions
        .iter()
        .filter(|x| check_key(biblio, x.1))
        .collect::<Vec<_>>();
    let actions_labels = actionsf.iter().map(|a| a.0).collect::<Vec<_>>();

    let options: SkimOptions = SkimOptions::default().multi(false);
    let selected_items = Skim::run_with(
        &options,
        Some(Box::new(Cursor::new(actions_labels.join("\n")))),
    ).map(|out| out.selected_items)
        .unwrap_or_else(|| Vec::new());

    for item in selected_items.iter() {
        actionsf[item.get_index()].2(biblio, settings);
    }
}

fn to_pbibliography<'a>(biblio: &'a Bibliography) -> PBibliography<'a> {
    let mut map = HashMap::new();
    for tag in biblio.tags() {
        let (k, v) = tag;
        let lk = k.to_ascii_lowercase();
        map.insert(lk, v);
    }

    return PBibliography {
        bib: biblio,
        fields: map,
    };
}

fn locate_bibs(path: &Path) -> HashMap<String, PathBuf> {
    let mut map = HashMap::new();
    for entry in glob(path.join("**").join("*.bib").to_str().unwrap())
        .expect("Failed to read glob pattern")
    {
        if let Ok(path) = entry {
            map.insert(path.file_name().unwrap().to_str().unwrap().to_owned(), path);
        }
    }
    return map;
}

fn main() {
    let app = App::new("bibfzf")
        .version("18.12")
        .arg(
            Arg::with_name("key")
                .short("k")
                .long("key")
                .help("Key to display from bibtex file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Configuration file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("bibtex")
                .help("Input bibtex file")
                .required(true)
                .value_name("BIBTEX"),
        );

    let matches = app.get_matches();

    let mut settings = Config::default();
    settings
        .set_default(
            "preamble",
            "
    @String { jan = \"January\" }
    @String { feb = \"February\" }
    @String { mar = \"March\" }
    @String { apr = \"April\" }
    @String { mai = \"Mai\" }
    @String { jun = \"June\" }
    @String { jul = \"July\" }
    @String { aug = \"August\" }
    @String { sep = \"September\" }
    @String { oct = \"October\" }
    @String { mov = \"November\" }
    @String { dec = \"December\" }",
        )
        .unwrap();
    settings.set_default("actions.open_doi", "open").unwrap();
    settings.set_default("actions.open_pdf", "open").unwrap();
    settings.set_default("actions.open_url", "open").unwrap();
    settings.set_default("actions.copy_cite", "pbcopy").unwrap();
    settings.set_default("actions.copy_key", "pbcopy").unwrap();
    settings
        .set_default("texlive_path", "/usr/local/texlive")
        .unwrap();

    let mut conffile = dirs::home_dir().unwrap().join(".bibfzf.conf");
    if let Some(conf) = matches.value_of("config") {
        conffile = Path::new(conf).to_path_buf();
    }
    if conffile.exists() {
        settings
            .merge(config::File::new(
                conffile.to_str().unwrap(),
                config::FileFormat::Toml,
            ))
            .unwrap();
    }

    let mut bib_str = settings.get_str("preamble").unwrap();
    bib_str.push_str("\n");

    if let Ok(pfiles) = settings.get_array("preamble_files") {
        let texlive_bibtex_path = Path::new(&settings.get_str("texlive_path").unwrap())
            .join("*")
            .join("texmf-dist")
            .join("bibtex")
            .join("bib");
        let texlive_bibtexs = locate_bibs(&texlive_bibtex_path);

        for pf in pfiles {
            if let Ok(pff) = pf.into_str() {
                if pff.starts_with("/") {
                    bib_str.push_str(&read_file(&pff));
                } else {
                    match texlive_bibtexs.get(&pff) {
                        Some(path) => bib_str.push_str(&read_file(path.to_str().unwrap())),
                        None => println!("Couldn't locate: {}", pff),
                    }
                }
            }
        }
    }

    let bib_path = matches.value_of("bibtex").unwrap();
    bib_str.push_str(&read_file(bib_path));

    // Parse bibtex data
    let bibtexr = Bibtex::parse(&bib_str);
    if !bibtexr.is_ok() {
        return;
    }
    let bibtex = bibtexr.unwrap();

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

    let pbibtex = bibtex
        .bibliographies()
        .iter()
        .map(|x| to_pbibliography(x))
        .collect::<Vec<_>>();

    let empty = "".to_string();
    let pfields: Vec<_> = ["title", "author", "year"]
        .iter()
        .map(|x| x.to_string())
        .collect();
    let mut v: Vec<String> = Vec::new();
    for entry in pbibtex.iter() {
        v.push(format!(
            "{} | {}",
            entry.bib.citation_key(),
            pfields
                .iter()
                .map(|k| {
                    strformat(entry.fields.get(k).unwrap_or(&&empty).to_owned())
                })
                .collect::<Vec<_>>()
                .join(" - ")
        ));
    }

    // Display list of keys using skim
    let preview = format!(
        "{} --key {{1}} {}",
        std::env::current_exe().unwrap().to_str().unwrap(),
        bib_path
    );
    let options: SkimOptions = SkimOptions::default().multi(false).delimiter("|").preview(
        preview.as_str(),
    );
    let selected_items = Skim::run_with(&options, Some(Box::new(Cursor::new(v.join("\n")))))
        .map(|out| out.selected_items)
        .unwrap_or_else(|| Vec::new());


    // Perform action on select entry
    for item in selected_items.iter() {
        let bibitem = &pbibtex[item.get_index()];
        actions_menu(bibitem, &settings);
    }
}
