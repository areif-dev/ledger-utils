mod transaction;

use chrono::{Local, TimeZone};
use clap::Parser;
use regex::Regex;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;
use std::{error::Error, path::PathBuf};
use transaction::{LineItem, LineItemBuilderError, TransactionBuilder, TransactionBuilderError};

#[derive(Debug)]
pub enum LedgerError {
    TransactionBuilder(TransactionBuilderError),
    LineItemBuilder(LineItemBuilderError),
    IoError(std::io::Error),
    MinijinjaError(minijinja::Error),
    Misc(String),
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for LedgerError {}

impl From<minijinja::Error> for LedgerError {
    fn from(value: minijinja::Error) -> Self {
        Self::MinijinjaError(value)
    }
}

impl From<TransactionBuilderError> for LedgerError {
    fn from(value: TransactionBuilderError) -> Self {
        Self::TransactionBuilder(value)
    }
}

impl From<LineItemBuilderError> for LedgerError {
    fn from(value: LineItemBuilderError) -> Self {
        Self::LineItemBuilder(value)
    }
}

impl From<std::io::Error> for LedgerError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short = 'f', long, value_name = "FILE")]
    journal: Option<PathBuf>,

    #[arg(short, long, value_name = "FILE")]
    template: PathBuf,

    #[arg(short, long)]
    date: Option<String>,

    #[arg(short = 'D', long)]
    desc: String,

    #[arg(short, long = "var")]
    vars: Vec<String>,

    #[arg(short, long = "override", value_name = "ENTRY")]
    overrides: Vec<String>,
}

impl Cli {
    pub fn get_date(&self) -> chrono::DateTime<Local> {
        let d = match &self.date {
            None => return chrono::Local::now(),
            Some(d) => d,
        };
        let nd = match chrono::NaiveDateTime::parse_from_str(&d, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => return chrono::Local::now(),
        };
        match Local.from_local_datetime(&nd) {
            chrono::offset::LocalResult::None => chrono::Local::now(),
            chrono::offset::LocalResult::Single(a) => a,
            chrono::offset::LocalResult::Ambiguous(a, _) => a,
        }
    }

    pub fn get_journal(&self) -> Result<PathBuf, std::env::VarError> {
        if let Some(j) = &self.journal {
            return Ok(j.to_path_buf());
        }

        Ok(PathBuf::from(std::env::var("LEDGER_FILE")?))
    }

    pub fn parse_vars(&self) -> (HashMap<String, minijinja::Value>, Vec<String>) {
        let mut bad_vars = Vec::new();
        let good_vars: HashMap<String, minijinja::Value> = self
            .vars
            .iter()
            .map(|v| {
                let mut split = v.split("=");
                (split.next(), split.last())
            })
            .filter_map(|(lhs, rhs)| match (lhs, rhs) {
                (Some(l), Some(r)) => match r.parse::<i64>() {
                    Ok(f) => Some((l.trim().to_string(), minijinja::Value::from(f))),
                    Err(_) => Some((l.trim().to_string(), minijinja::Value::from(r))),
                },
                (None, Some(r)) => {
                    bad_vars.push(r.trim().to_string());
                    None
                }
                (Some(l), None) => {
                    bad_vars.push(l.to_string());
                    None
                }
                (None, None) => {
                    bad_vars.push("".to_string());
                    None
                }
            })
            .collect();
        (good_vars, bad_vars)
    }
}

fn get_balance(account: &str, journal: &Path) -> Result<i64, LedgerError> {
    let stdout = match std::process::Command::new("hledger")
        .arg("-f")
        .arg(journal.as_os_str())
        .arg("bal")
        .arg(account)
        .output()
    {
        Ok(s) => s,
        Err(_) => {
            match std::process::Command::new("ledger")
                .arg("-f")
                .arg(journal.as_os_str())
                .arg("bal")
                .arg(account)
                .output()
            {
                Ok(s) => s,
                Err(e) => {
                    return Err(LedgerError::Misc(format!(
                        "Failed to execute hledger and ledger commands. Are they installed?: {}",
                        e
                    )))
                }
            }
        }
    }
    .stdout;
    let mut split = stdout.split(|c| char::from(*c) == '\n');
    let balance_bytes: Vec<u8> = match split.nth_back(1) {
        Some(b) => b,
        None => {
            return Err(LedgerError::Misc(format!(
                "Could not parse balance for account {}",
                account
            )))
        }
    }
    .iter()
    .filter_map(|c| {
        if char::from(*c).is_digit(10) || char::from(*c) == '-' || char::from(*c) == '.' {
            Some(*c)
        } else {
            None
        }
    })
    .collect();
    let balance_str = String::from_utf8_lossy(&balance_bytes);
    let balance_f64: f64 = balance_str.parse().or(Err(LedgerError::Misc(format!(
        "Could not parse f64 for balance of account {}",
        account
    ))))?;
    Ok((balance_f64 * 100.0).round() as i64)
}

fn render_balances(template_str: &str, journal: PathBuf) -> Result<String, LedgerError> {
    let regex = Regex::new("<<.*>>").unwrap();
    let accounts: Vec<&str> = regex
        .find_iter(&template_str)
        .filter_map(|m| {
            let sub = m.as_str();
            sub.get(2..sub.len() - 2)
        })
        .collect();
    let mut fixed_template = template_str.to_owned();
    for acct in &accounts {
        let balance = get_balance(acct, journal.as_path())?;
        fixed_template = fixed_template.replace(&format!("<<{}>>", acct), &balance.to_string());
    }
    Ok(fixed_template)
}

fn render_tempate(
    template_file: PathBuf,
    journal: PathBuf,
    ctx: minijinja::Value,
) -> Result<Vec<LineItem>, LedgerError> {
    let template_env = minijinja::Environment::new();
    let template_str = std::fs::read_to_string(template_file)?;
    let template_str = render_balances(&template_str, journal)?;
    let render = template_env.render_str(&template_str, ctx)?;
    let mut lines = Vec::new();
    for line in render.lines() {
        lines.push(line.try_into()?);
    }
    Ok(lines)
}

fn add_overrides(
    defaults: &mut Vec<LineItem>,
    overrides: &mut Vec<LineItem>,
) -> Result<Vec<LineItem>, LedgerError> {
    overrides.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    defaults.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let mut lines: Vec<LineItem> = Vec::new();
    let mut defaults = defaults.iter();
    let mut overrides = overrides.iter();
    let mut default = defaults.next();
    let mut over = overrides.next();
    loop {
        match (over, default) {
            (None, None) => break,
            (Some(o), Some(d)) => match o.partial_cmp(d) {
                Some(Ordering::Less) => {
                    lines.push(o.to_owned());
                    over = overrides.next();
                }
                Some(Ordering::Equal) => {
                    lines.push(o.to_owned());
                    over = overrides.next();
                    default = defaults.next();
                }
                Some(Ordering::Greater) => {
                    lines.push(d.to_owned());
                    default = defaults.next();
                }
                None => {
                    return Err(LedgerError::TransactionBuilder(
                        TransactionBuilderError::DoesNotBalance(-1),
                    ))
                }
            },
            (Some(o), None) => {
                lines.push(o.to_owned());
                over = overrides.next();
            }
            (None, Some(d)) => {
                lines.push(d.to_owned());
                default = defaults.next();
            }
        }
    }
    Ok(lines)
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let (vars, bad_vars) = cli.parse_vars();
    for var in bad_vars {
        eprintln!(
            "Skipping bad var \"{}\". Vars should be passed in the form of `-v name=value`",
            var
        );
    }
    let journal = cli.get_journal()?;
    let line_items =
        match render_tempate(cli.template.as_path().to_path_buf(), journal, vars.into()) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Failed to parse template because of {:?}", e);
                return Err(e)?;
            }
        };
    let transaction = match TransactionBuilder::new()
        .date(cli.get_date())
        .desc(cli.desc)
        .line_items(line_items)
        .balance()
    {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Could not build transaction because of {:?}", e);
            return Err(e)?;
        }
    };
    println!("{}", transaction);
    Ok(())
}
