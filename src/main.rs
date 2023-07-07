use anyhow::{Context, Result};
use clap::Parser;
use lazy_static::lazy_static;
use log::debug;
use owo_colors::{self, OwoColorize, Style};
use regex::Regex;
use std::{
    collections::HashMap,
    io::{stdin, stdout, BufRead, Write},
};

#[derive(Parser, Clone, Debug, Default)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The regular expression to match against.
    #[arg(required = true)]
    regex: String,

    /// Color map.
    #[arg(value_parser = parse_colors)]
    colors: Option<HashMap<String, Style>>,
}

lazy_static! {
    static ref COLORS: HashMap<String, Style> = {
        let mut map = HashMap::new();
        map.insert("black".to_string(), Style::new().black());
        map.insert("red".to_string(), Style::new().red());
        map.insert("green".to_string(), Style::new().green());
        map.insert("yellow".to_string(), Style::new().yellow());
        map.insert("blue".to_string(), Style::new().blue());
        map.insert("magenta".to_string(), Style::new().magenta());
        map.insert("cyan".to_string(), Style::new().cyan());
        map.insert("white".to_string(), Style::new().white());
        map.insert("bright_black".to_string(), Style::new().bright_black());
        map.insert("bright_red".to_string(), Style::new().bright_red());
        map.insert("bright_green".to_string(), Style::new().bright_green());
        map.insert("bright_yellow".to_string(), Style::new().bright_yellow());
        map.insert("bright_blue".to_string(), Style::new().bright_blue());
        map.insert("bright_magenta".to_string(), Style::new().bright_magenta());
        map.insert("bright_cyan".to_string(), Style::new().bright_cyan());
        map.insert("bright_white".to_string(), Style::new().bright_white());
        map
    };
    static ref COLOR_VEC: Vec<Style> = {
        let mut vec = Vec::new();
        for (_, color) in COLORS.iter() {
            vec.push(*color);
        }
        vec
    };
}

fn parse_color(s: &str) -> Result<Style> {
    COLORS
        .get(s)
        .copied()
        .ok_or_else(|| anyhow::anyhow!(format!("invalid color \"{}\"", s)))
}

fn parse_colors(s: &str) -> Result<HashMap<String, Style>> {
    let mut map = HashMap::new();
    for pair in s.split(',') {
        let mut pair = pair.split('=');
        let key = pair
            .next()
            .context("invalid colors, format is key=value,key=value")?;
        let value = pair
            .next()
            .context("invalid colors, format is key=value,key=value")?;
        let color = parse_color(value)?;
        map.insert(key.to_string(), color);
    }
    Ok(map)
}

fn run(input: impl BufRead, mut output: impl Write, args: Args) -> Result<()> {
    let regex = Regex::new(&args.regex).context("invalid regex")?;
    let colors = args.colors.unwrap_or_default();

    for line in input.lines() {
        let line = line?;
        let mut last_match_end = 0;
        for m in regex.captures_iter(&line) {
            for (i, capture) in m.iter().enumerate().skip(1) {
                let color = match regex.capture_names().nth(i) {
                    Some(Some(name)) => colors
                        .get(name)
                        .copied()
                        .unwrap_or(COLOR_VEC[i % COLOR_VEC.len()]),
                    _ => COLOR_VEC[i % COLOR_VEC.len()],
                };
                if let Some(mat) = capture {
                    write!(
                        output,
                        "{}{}",
                        &line[last_match_end..mat.start()],
                        &line[mat.start()..mat.end()].to_owned().style(color)
                    )?;
                    last_match_end = mat.end();
                }
            }
        }
        if last_match_end < line.len() {
            write!(output, "{}", &line[last_match_end..])?;
        }
        write!(output, "\n")?;
    }

    Ok(())
}

fn main() -> Result<()> {
    human_panic::setup_panic!();
    env_logger::init();

    let args = Args::parse();
    debug!("args: {:?}", args);

    run(stdin().lock(), stdout().lock(), args)
}
