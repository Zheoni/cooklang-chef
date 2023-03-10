use std::{borrow::Cow, ops::Range};

use thiserror::Error;

pub type CookResult<T> = Result<T, CooklangReport>;
pub type CooklangReport = Report<CooklangError, CooklangWarning>;

#[derive(Debug, Clone)]
pub struct Report<E, W> {
    errors: Vec<E>,
    warnings: Vec<W>,
}

impl<E, W> Report<E, W>
where
    E: RichError,
    W: RichError,
{
    pub fn new(errors: Vec<E>, warnings: Vec<W>) -> Self {
        Self { errors, warnings }
    }
    pub fn from_err(error: E) -> Self {
        Self {
            errors: vec![error],
            warnings: vec![],
        }
    }
    pub fn from_warning(warning: W) -> Self {
        Self {
            errors: vec![],
            warnings: vec![warning],
        }
    }
    pub fn from_report<E2, W2>(other: Report<E2, W2>) -> Self
    where
        E2: Into<E>,
        W2: Into<W>,
    {
        Self {
            errors: other.errors.into_iter().map(Into::into).collect(),
            warnings: other.warnings.into_iter().map(Into::into).collect(),
        }
    }

    pub fn write(
        &self,
        cache: &mut impl ariadne::Cache<()>,
        w: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        for warn in &self.warnings {
            warn.write(cache, w)?;
        }
        for err in &self.errors {
            err.write(cache, w)?;
        }
        Ok(())
    }
    pub fn print(&self, file_name: &str, source_code: &str) -> std::io::Result<()> {
        let mut cache = DummyCache::new(file_name, source_code);
        self.write(&mut cache, &mut std::io::stdout())
    }
    pub fn eprint(&self, file_name: &str, source_code: &str) -> std::io::Result<()> {
        let mut cache = DummyCache::new(file_name, source_code);
        self.write(&mut cache, &mut std::io::stderr())
    }
}

impl<E, W> std::fmt::Display for Report<E, W>
where
    E: std::fmt::Display,
    W: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let errors = &self.errors;
        let warnings = &self.warnings;
        if errors.len() == 1 {
            errors[0].fmt(f)?;
        } else if warnings.len() == 1 {
            warnings[0].fmt(f)?;
        } else {
            match (errors.is_empty(), warnings.is_empty()) {
                (true, true) => writeln!(f, "Unknown error")?,
                (true, false) => writeln!(f, "Multiple warnings:")?,
                (false, _) => writeln!(f, "Multiple errors:")?,
            }
            for warn in warnings {
                warn.fmt(f)?;
            }
            for err in errors {
                err.fmt(f)?;
            }
        }
        Ok(())
    }
}
impl<E, W> std::error::Error for Report<E, W>
where
    E: std::fmt::Display + std::fmt::Debug,
    W: std::fmt::Display + std::fmt::Debug,
{
}

#[derive(Debug, Error)]
pub enum CooklangError {
    #[error(transparent)]
    Parser(#[from] crate::parser::ParserError),
    #[error(transparent)]
    Analysis(#[from] crate::analysis::AnalysisError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("No file name in path: '{path}'")]
    NoFilename { path: std::path::PathBuf },
}

#[derive(Debug, Error)]
#[error(transparent)]
pub enum CooklangWarning {
    Parser(#[from] crate::parser::ParserWarning),
    Analysis(#[from] crate::analysis::AnalysisWarning),
}

impl RichError for CooklangError {
    fn labels(&self) -> Vec<(Range<usize>, Option<Cow<'static, str>>)> {
        match self {
            CooklangError::Parser(e) => e.labels(),
            CooklangError::Analysis(e) => e.labels(),
            CooklangError::Io(_) => vec![],
            CooklangError::NoFilename { .. } => vec![],
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        match self {
            CooklangError::Parser(e) => e.help(),
            CooklangError::Analysis(e) => e.help(),
            CooklangError::Io(_) => None,
            CooklangError::NoFilename { .. } => {
                help!("The recipe name is needed and comes from the file name")
            }
        }
    }

    fn code(&self) -> Option<&'static str> {
        match self {
            CooklangError::Parser(e) => e.code(),
            CooklangError::Analysis(e) => e.code(),
            CooklangError::Io(_) => Some("io"),
            CooklangError::NoFilename { .. } => Some("no_file_name"),
        }
    }
}

impl RichError for CooklangWarning {
    fn labels(&self) -> Vec<(Range<usize>, Option<Cow<'static, str>>)> {
        match self {
            CooklangWarning::Parser(e) => e.labels(),
            CooklangWarning::Analysis(e) => e.labels(),
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        match self {
            CooklangWarning::Parser(e) => e.help(),
            CooklangWarning::Analysis(e) => e.help(),
        }
    }

    fn code(&self) -> Option<&'static str> {
        match self {
            CooklangWarning::Parser(e) => e.code(),
            CooklangWarning::Analysis(e) => e.code(),
        }
    }

    fn kind(&self) -> ariadne::ReportKind {
        ariadne::ReportKind::Warning
    }
}

pub fn print_warnings(file_name: &str, source_code: &str, warnings: &[CooklangWarning]) {
    let mut cache = DummyCache::new(file_name, source_code);
    let mut stderr = std::io::stderr();
    for w in warnings {
        w.write(&mut cache, &mut stderr).unwrap()
    }
}

pub trait RichError: std::error::Error {
    fn labels(&self) -> Vec<(Range<usize>, Option<Cow<'static, str>>)> {
        vec![]
    }
    fn help(&self) -> Option<Cow<'static, str>> {
        None
    }
    fn code(&self) -> Option<&'static str> {
        None
    }
    fn kind(&self) -> ariadne::ReportKind {
        ariadne::ReportKind::Error
    }
    fn offset(&self) -> Option<usize> {
        None
    }
}

macro_rules! label {
    ($span:expr) => {
        ($span.to_owned(), None)
    };
    ($span:expr, $message:expr) => {
        ($span.to_owned().into(), Some($message.into()))
    };
}
pub(crate) use label;

macro_rules! help {
    () => {
        None
    };
    ($help:expr) => {
        Some($help.into())
    };
    (opt $help:expr) => {
        $help.map(|h| h.into())
    };
}
pub(crate) use help;

fn build_report(err: &dyn RichError) -> ariadne::Report {
    use ariadne::{Color, ColorGenerator, Fmt, Label, Report};

    let labels = err.labels();
    let offset = err
        .offset()
        .or_else(|| err.labels().first().map(|l| l.0.start))
        .unwrap_or_default();

    let mut r = Report::build(err.kind(), (), offset);

    if let Some(source) = err.source() {
        let color = match err.kind() {
            ariadne::ReportKind::Error => Color::Red,
            ariadne::ReportKind::Warning => Color::Yellow,
            ariadne::ReportKind::Advice => Color::Fixed(147),
            ariadne::ReportKind::Custom(_, c) => c,
        };
        let message = format!("{err}\n  {} {source}", "╰▶ ".fg(color));
        r.set_message(message);
    } else {
        r.set_message(err);
    }

    let mut c = ColorGenerator::new();
    r.add_labels(labels.into_iter().map(|(span, text)| {
        let mut l = Label::new(span).with_color(c.next());
        if let Some(text) = text {
            l = l.with_message(text);
        }
        l
    }));

    if let Some(help) = err.help() {
        r = r.with_help(help);
    }

    r.finish()
}

pub struct DummyCache(String, ariadne::Source);
impl DummyCache {
    fn new(file_name: &str, src_code: &str) -> Self {
        Self(file_name.into(), src_code.into())
    }
}
impl ariadne::Cache<()> for DummyCache {
    fn fetch(&mut self, _id: &()) -> Result<&ariadne::Source, Box<dyn std::fmt::Debug + '_>> {
        Ok(&self.1)
    }

    fn display<'a>(&self, _id: &'a ()) -> Option<Box<dyn std::fmt::Display + 'a>> {
        Some(Box::new(self.0.clone()))
    }
}

pub trait WriteRichError<C: ariadne::Cache<()> = DummyCache> {
    fn write(&self, cache: &mut C, w: &mut impl std::io::Write) -> std::io::Result<()>;
    fn print(&self, cache: &mut C) -> std::io::Result<()> {
        self.write(cache, &mut std::io::stdout())
    }
    fn eprint(&self, cache: &mut C) -> std::io::Result<()> {
        self.write(cache, &mut std::io::stderr())
    }
}

impl<C, E> WriteRichError<C> for E
where
    E: RichError,
    C: ariadne::Cache<()>,
{
    fn write(&self, cache: &mut C, w: &mut impl std::io::Write) -> std::io::Result<()> {
        build_report(self).write(cache, w)
    }
}
