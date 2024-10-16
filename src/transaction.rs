use std::{cmp::Ordering, fmt::Display, fs::OpenOptions, io::Write, path::PathBuf};

use chrono::Local;

#[derive(Debug)]
pub struct LineItem {
    pub account: String,
    pub value: i64,
    pub is_real: bool,
}

#[derive(Debug)]
pub struct LineItemBuilder {
    account: Option<String>,
    value: Option<i64>,
    is_real: Option<bool>,
}

#[derive(Debug)]
pub enum LineItemBuilderError {
    MissingAccount,
    MissingValue,
    MissingIsReal,
}

impl std::error::Error for LineItemBuilderError {}

impl std::fmt::Display for LineItemBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl LineItemBuilder {
    pub fn new() -> Self {
        Self {
            account: None,
            value: None,
            is_real: None,
        }
    }

    pub fn account<S>(self, name: S) -> Self
    where
        S: ToString,
    {
        Self {
            account: Some(name.to_string()),
            ..self
        }
    }

    pub fn value(self, value: i64) -> Self {
        Self {
            value: Some(value),
            ..self
        }
    }

    pub fn is_real(self, is_real: bool) -> Self {
        Self {
            is_real: Some(is_real),
            ..self
        }
    }

    pub fn try_build(self) -> Result<LineItem, LineItemBuilderError> {
        let account = self.account.ok_or(LineItemBuilderError::MissingAccount)?;
        let value = self.value.ok_or(LineItemBuilderError::MissingValue)?;
        let is_real = self.is_real.ok_or(LineItemBuilderError::MissingIsReal)?;
        Ok(LineItem {
            account,
            value,
            is_real,
        })
    }
}

impl ToOwned for LineItem {
    type Owned = Self;

    fn to_owned(&self) -> Self::Owned {
        Self {
            account: self.account.to_string(),
            value: self.value,
            is_real: self.is_real,
        }
    }

    fn clone_into(&self, target: &mut Self::Owned) {
        target.account = self.account.to_string();
        target.value = self.value;
        target.is_real = self.is_real;
    }
}

impl PartialEq for LineItem {
    fn eq(&self, other: &Self) -> bool {
        self.is_real == other.is_real && self.account == self.account && self.value == other.value
    }
}

impl PartialOrd for LineItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self.is_real, other.is_real) {
            (true, false) => return Some(Ordering::Less),
            (false, true) => return Some(Ordering::Greater),
            _ => (),
        }

        self.account.partial_cmp(&other.account)
    }
}

impl TryFrom<&str> for LineItem {
    type Error = LineItemBuilderError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut split = value.split("  ");
        let lhs = split
            .next()
            .ok_or(LineItemBuilderError::MissingAccount)?
            .trim();
        let rhs = split
            .last()
            .ok_or(LineItemBuilderError::MissingValue)?
            .trim();
        let is_real = match (lhs.get(0..1), lhs.get(lhs.len() - 1..)) {
            (Some("["), Some("]")) => false,
            (_, None) | (None, _) | (Some("["), Some(_)) | (Some(_), Some("]")) => {
                return Err(LineItemBuilderError::MissingIsReal)
            }
            _ => true,
        };
        let account = if is_real {
            lhs.to_string()
        } else {
            match lhs.get(1..lhs.len() - 1) {
                Some(s) => s.to_string(),
                None => return Err(LineItemBuilderError::MissingAccount),
            }
        };
        let rhs: String = rhs.chars().filter(|c| *c != '$').collect();
        let value: f64 = rhs.parse().or(Err(LineItemBuilderError::MissingValue))?;
        Ok(LineItem {
            account,
            value: (value * 100.0).round() as i64,
            is_real,
        })
    }
}

impl TryFrom<String> for LineItem {
    type Error = LineItemBuilderError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut split = value.split("  ");
        let lhs = split
            .next()
            .ok_or(LineItemBuilderError::MissingAccount)?
            .trim();
        let rhs = split
            .last()
            .ok_or(LineItemBuilderError::MissingValue)?
            .trim();
        let is_real = match (lhs.get(0..1), lhs.get(lhs.len() - 1..)) {
            (Some("["), Some("]")) => false,
            (_, None) | (None, _) | (Some("["), Some(_)) | (Some(_), Some("]")) => {
                return Err(LineItemBuilderError::MissingIsReal)
            }
            _ => true,
        };
        let account = if is_real {
            lhs.to_string()
        } else {
            match lhs.get(1..lhs.len() - 1) {
                Some(s) => s.to_string(),
                None => return Err(LineItemBuilderError::MissingAccount),
            }
        };
        let rhs: String = rhs.chars().filter(|c| *c != '$').collect();
        let value: i64 = rhs.parse().or(Err(LineItemBuilderError::MissingValue))?;
        Ok(LineItem {
            account,
            value,
            is_real,
        })
    }
}

impl TryFrom<LineItemBuilder> for LineItem {
    type Error = LineItemBuilderError;
    fn try_from(value: LineItemBuilder) -> Result<Self, Self::Error> {
        value.try_build()
    }
}

impl Display for LineItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let full_name = if self.is_real {
            self.account.to_owned()
        } else {
            format!("[{}]", self.account)
        };
        let value_float = self.value as f64 / 100.0;
        write!(f, "{}  \t${:.02}", full_name, value_float)
    }
}

#[derive(Debug)]
pub struct Transaction {
    date: chrono::DateTime<Local>,
    desc: String,
    line_items: Vec<LineItem>,
}

impl Transaction {
    pub fn post(&self, file: PathBuf) -> Result<(), std::io::Error> {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(file)?;
        writeln!(file, "{}", self)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct TransactionBuilder {
    date: Option<chrono::DateTime<Local>>,
    desc: Option<String>,
    line_items: Vec<LineItem>,
}

#[derive(Debug)]
pub enum TransactionBuilderError {
    MissingDate,
    MissingDesc,
    NotEnoughLineItems,
    DoesNotBalance(i64),
}

impl std::fmt::Display for TransactionBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for TransactionBuilderError {}

impl TransactionBuilder {
    pub fn new() -> Self {
        Self {
            date: None,
            desc: None,
            line_items: Vec::new(),
        }
    }

    pub fn date(self, date: chrono::DateTime<Local>) -> Self {
        Self {
            date: Some(date),
            ..self
        }
    }

    pub fn desc<S>(self, desc: S) -> Self
    where
        S: ToString,
    {
        Self {
            desc: Some(desc.to_string()),
            ..self
        }
    }

    pub fn line_items(self, line_items: Vec<LineItem>) -> Self {
        Self { line_items, ..self }
    }

    pub fn add_line(self, line: LineItem) -> Self {
        let mut lines = self.line_items;
        lines.push(line);
        Self {
            line_items: lines,
            ..self
        }
    }

    pub fn current_virt_balance(&self) -> i64 {
        self.line_items
            .iter()
            .filter(|l| !l.is_real)
            .map(|l| l.value)
            .sum()
    }

    pub fn current_real_balance(&self) -> i64 {
        self.line_items
            .iter()
            .filter(|l| l.is_real)
            .map(|l| l.value)
            .sum()
    }

    pub fn balance(self) -> Result<Transaction, TransactionBuilderError> {
        let date = self.date.ok_or(TransactionBuilderError::MissingDate)?;
        let desc = self.desc.ok_or(TransactionBuilderError::MissingDesc)?;
        if self.line_items.len() < 2 {
            return Err(TransactionBuilderError::NotEnoughLineItems);
        }

        let virt_balance: i64 = self
            .line_items
            .iter()
            .filter(|l| !l.is_real)
            .map(|l| l.value)
            .sum();
        if virt_balance != 0 {
            return Err(TransactionBuilderError::DoesNotBalance(virt_balance));
        }
        let real_balance: i64 = self
            .line_items
            .iter()
            .filter(|l| l.is_real)
            .map(|l| l.value)
            .sum();
        if real_balance != 0 {
            return Err(TransactionBuilderError::DoesNotBalance(real_balance));
        }

        Ok(Transaction {
            date,
            desc,
            line_items: self.line_items,
        })
    }
}

impl TryFrom<TransactionBuilder> for Transaction {
    type Error = TransactionBuilderError;
    fn try_from(value: TransactionBuilder) -> Result<Self, Self::Error> {
        value.balance()
    }
}

impl Display for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let date_str = self.date.format("%Y-%m-%d");
        let lines = self
            .line_items
            .iter()
            .map(|l| format!("    {}", l))
            .collect::<Vec<String>>()
            .join("\n");
        write!(f, "{} {}\n{}", date_str, self.desc, lines)
    }
}
