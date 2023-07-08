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
    /// A regular expression to match each line of the output piped to this
    /// program against. Each capture group will be styled with the color
    /// corresponding to the group name, or a default color based on the capture
    /// group index if the group has no name.
    #[arg(required = true)]
    regex: String,

    /// The rest of the arguments are key=value pairs, where the key is the name
    /// of the capture group, and the value is a comma-separated list of styles
    /// to apply to that capture group. The styles are applied in order, so
    /// `bold,red` will make the text bold and red, while `red,green` will make
    /// the text green.
    #[arg()]
    styles: Vec<String>,
}

lazy_static! {
    static ref DEFAULT_STYLES: Vec<Style> = {
        vec![
            Style::new().red(),
            Style::new().green(),
            Style::new().yellow(),
            Style::new().blue(),
            Style::new().magenta(),
            Style::new().cyan(),
            Style::new().white(),
        ]
    };
}

fn parse_style(s: &str) -> Result<Style> {
    let mut style = Style::new();
    for part in s.split(',') {
        if part.starts_with('#') {
            if part.len() != 7 {
                return Err(anyhow::anyhow!(format!("invalid hex color: \"{}\"", s)));
            }
            let (r, g, b) = (
                u8::from_str_radix(&part[1..3], 16)?,
                u8::from_str_radix(&part[3..5], 16)?,
                u8::from_str_radix(&part[5..7], 16)?,
            );
            style = style.truecolor(r, g, b);
            continue;
        }
        style = match part {
            "black" => style.black(),
            "red" => style.red(),
            "green" => style.green(),
            "yellow" => style.yellow(),
            "blue" => style.blue(),
            "magenta" => style.magenta(),
            "cyan" => style.cyan(),
            "white" => style.white(),
            "bright_black" => style.bright_black(),
            "bright_red" => style.bright_red(),
            "bright_green" => style.bright_green(),
            "bright_yellow" => style.bright_yellow(),
            "bright_blue" => style.bright_blue(),
            "bright_magenta" => style.bright_magenta(),
            "bright_cyan" => style.bright_cyan(),
            "bright_white" => style.bright_white(),
            "bold" | "bolded" => style.bold(),
            "dimmed" | "dim" => style.dimmed(),
            "italic" | "italics" => style.italic(),
            "underline" | "underlined" => style.underline(),
            "blink" | "blinking" => style.blink(),
            "hidden" => style.hidden(),
            "strikethrough" | "struckthrough" | "strike" => style.strikethrough(),
            _ => return Err(anyhow::anyhow!(format!("invalid style: \"{}\"", s))),
        };
    }
    Ok(style)
}

fn parse_styles(styles: Vec<String>) -> Result<HashMap<String, Style>> {
    let mut map = HashMap::new();
    for style in styles {
        let mut pair = style.split('=');
        let key = pair
            .next()
            .context("invalid styles, format is key=value,key=value")?;
        let value = pair
            .next()
            .context("invalid styles, format is key=value,key=value")?;
        let style = parse_style(value)?;
        map.insert(key.to_string(), style);
    }
    Ok(map)
}

enum Op {
    Push(Style),
    Pop,
}

fn run(input: impl BufRead, mut output: impl Write, args: Args) -> Result<()> {
    let regex = Regex::new(&args.regex).context("invalid regex")?;
    let styles = parse_styles(args.styles)?;

    let mut ops_by_position: HashMap<usize, Vec<Op>> = HashMap::new();
    let mut style_stack: Vec<Style> = Vec::new();

    for line in input.lines() {
        ops_by_position.clear();
        style_stack.clear();

        let line = line?;
        for m in regex.captures_iter(&line) {
            for (i, capture) in m.iter().enumerate().skip(1) {
                let style = match regex.capture_names().nth(i) {
                    Some(Some(name)) => styles
                        .get(name)
                        .copied()
                        .unwrap_or(DEFAULT_STYLES[i % DEFAULT_STYLES.len()]),
                    _ => DEFAULT_STYLES[i % DEFAULT_STYLES.len()],
                };

                if let Some(mat) = capture {
                    ops_by_position
                        .entry(mat.start())
                        .or_default()
                        .push(Op::Push(style));

                    ops_by_position.entry(mat.end()).or_default().push(Op::Pop);
                }
            }
        }

        let mut buf = String::new();
        for (position, char) in line.char_indices() {
            if let Some(ops) = ops_by_position.get(&position) {
                let style = style_stack.last().copied().unwrap_or_default();
                write!(output, "{}", buf.style(style))?;
                buf.clear();

                for op in ops {
                    match op {
                        Op::Push(style) => style_stack.push(*style),
                        Op::Pop => {
                            style_stack.pop();
                        }
                    }
                }
            }
            buf.push(char);
        }
        let style = style_stack.last().copied().unwrap_or_default();
        write!(output, "{}", buf.style(style))?;
        writeln!(output)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use test_case::test_case;

    #[test_case(
        vec!["(foo)"],
        "hello foo",
        format!("hello {}\n", "foo".style(DEFAULT_STYLES[1]))
        ; "single match")
    ]
    #[test_case(
        vec!["(foo)(bar)"],
        "hello foobar",
        format!("hello {}{}\n", "foo".style(DEFAULT_STYLES[1]), "bar".style(DEFAULT_STYLES[2]))
        ; "multiple match")
    ]
    #[test_case(
        vec!["(?P<foo>foo)(?P<bar>bar)", "foo=green", "bar=red"],
        "hello foobar",
        format!(
            "hello {}{}\n",
            "foo".style(Style::new().green()),
            "bar".style(Style::new().red())
        )
        ; "named matches")
    ]
    #[test_case(
        vec!["(5)"],
        "12345 12345 12345",
        format!(
            "1234{0} 1234{0} 1234{0}\n",
            "5".style(DEFAULT_STYLES[1]),
        )
        ; "multiple single match")
    ]
    #[test_case(
        vec!["(5)"],
        "hello world",
        "hello world\n"
        ; "no matches")
    ]
    #[test_case(
        vec!["(?P<five>5)", "five=#ff0000,underline"],
        "12345 12345 12345",
        format!(
            "1234{0} 1234{0} 1234{0}\n",
            "5".style(Style::new().truecolor(255, 0, 0).underline()),
        )
        ; "CSS colors")
    ]
    #[test_case(
        vec!["123(5)"],
        "12345 12345 1235",
        format!(
            "12345 12345 123{0}\n",
            "5".style(DEFAULT_STYLES[1]),
        )
        ; "regex with non-capture group component")
    ]
    #[test_case(
        vec!["12(3(5))"],
        "12345 12345 1235",
        format!(
            "12345 12345 12{}{}\n",
            "3".style(DEFAULT_STYLES[1]),
            "5".style(DEFAULT_STYLES[2]),
        )
        ; "capture group inside another capture group")
    ]
    fn test_success(
        args: impl Into<Vec<&'static str>>,
        input: impl Into<String>,
        expected_output: impl Into<String>,
    ) -> Result<()> {
        let mut output = Vec::new();
        let mut args: Vec<&str> = args.into();
        args.insert(0, "recolor");
        let args = Args::parse_from(args);
        run(Cursor::new(input.into()), &mut output, args)?;
        assert_eq!(String::from_utf8(output)?, expected_output.into());
        Ok(())
    }
}
