//! Error type, formatting and utilities.

use std::borrow::Cow;

use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Report<E, W> {
    pub(crate) errors: Vec<E>,
    pub(crate) warnings: Vec<W>,
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

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty() && self.warnings.is_empty()
    }

    pub fn write(
        &self,
        file_name: &str,
        source_code: &str,
        hide_warnings: bool,
        w: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        let mut cache = DummyCache::new(file_name, source_code);
        if !hide_warnings {
            for warn in &self.warnings {
                build_report(warn, source_code).write(&mut cache, &mut *w)?;
            }
        }
        for err in &self.errors {
            build_report(err, source_code).write(&mut cache, &mut *w)?;
        }
        Ok(())
    }
    pub fn print(
        &self,
        file_name: &str,
        source_code: &str,
        hide_warnings: bool,
    ) -> std::io::Result<()> {
        self.write(
            file_name,
            source_code,
            hide_warnings,
            &mut std::io::stdout(),
        )
    }
    pub fn eprint(
        &self,
        file_name: &str,
        source_code: &str,
        hide_warnings: bool,
    ) -> std::io::Result<()> {
        self.write(
            file_name,
            source_code,
            hide_warnings,
            &mut std::io::stderr(),
        )
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

#[derive(Debug)]
pub struct PassResult<T, E, W> {
    output: Option<T>,
    warnings: Vec<W>,
    errors: Vec<E>,
}

impl<T, E, W> PassResult<T, E, W> {
    pub fn new(output: Option<T>, warnings: Vec<W>, errors: Vec<E>) -> Self {
        Self {
            output,
            warnings,
            errors,
        }
    }

    pub(crate) fn from_error(error: E) -> Self {
        Self {
            output: None,
            warnings: vec![],
            errors: vec![error],
        }
    }

    pub fn has_output(&self) -> bool {
        self.output.is_some()
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn invalid(&self) -> bool {
        self.has_errors() || !self.has_output()
    }

    pub fn output(&self) -> Option<&T> {
        self.output.as_ref()
    }

    pub fn warnings(&self) -> &[W] {
        &self.warnings
    }

    pub fn errors(&self) -> &[E] {
        &self.errors
    }

    pub fn into_result(mut self) -> Result<(T, Report<E, W>), Report<E, W>> {
        if let Some(o) = self.output.take() {
            if self.errors.is_empty() {
                return Ok((o, self.into_report()));
            }
        }
        Err(self.into_report())
    }

    pub fn into_report(self) -> Report<E, W> {
        Report {
            errors: self.errors,
            warnings: self.warnings,
        }
    }

    pub fn take_output(&mut self) -> Option<T> {
        self.output.take()
    }

    pub fn into_output(self) -> Option<T> {
        self.output
    }

    pub fn into_errors(self) -> Vec<E> {
        self.errors
    }

    pub fn into_warnings(self) -> Vec<W> {
        self.warnings
    }

    pub fn into_tuple(self) -> (Option<T>, Vec<W>, Vec<E>) {
        (self.output, self.warnings, self.errors)
    }

    pub(crate) fn into_context_result<E2, W2>(self) -> PassResult<T, E2, W2>
    where
        E2: From<E>,
        W2: From<W>,
    {
        PassResult {
            output: self.output,
            errors: self.errors.into_iter().map(Into::into).collect(),
            warnings: self.warnings.into_iter().map(Into::into).collect(),
        }
    }

    pub(crate) fn discard_output<T2>(self) -> PassResult<T2, E, W> {
        PassResult {
            output: None,
            warnings: self.warnings,
            errors: self.errors,
        }
    }

    pub(crate) fn merge<T2>(mut self, mut other: PassResult<T2, E, W>) -> Self {
        other.errors.append(&mut self.errors);
        other.warnings.append(&mut self.warnings);
        self.errors = other.errors;
        self.warnings = other.warnings;
        self
    }

    pub fn map<F, O>(self, f: F) -> PassResult<O, E, W>
    where
        F: FnOnce(T) -> O,
    {
        PassResult {
            output: self.output.map(f),
            warnings: self.warnings,
            errors: self.errors,
        }
    }

    pub fn try_map<F, O, E2>(self, f: F) -> Result<PassResult<O, E, W>, E2>
    where
        F: FnOnce(T) -> Result<O, E2>,
    {
        let output = self.output.map(f).transpose()?;
        Ok(PassResult {
            output,
            warnings: self.warnings,
            errors: self.errors,
        })
    }
}

impl<T, E, W> From<E> for PassResult<T, E, W> {
    fn from(value: E) -> Self {
        Self::from_error(value)
    }
}

pub trait RichError<Id: Clone = ()>: std::error::Error {
    fn labels(&self) -> Vec<(Span<Id>, Option<Cow<'static, str>>)> {
        vec![]
    }
    fn help(&self) -> Option<Cow<'static, str>> {
        None
    }
    fn note(&self) -> Option<Cow<'static, str>> {
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
    fn source_id(&self) -> Option<Id> {
        None
    }
}

macro_rules! label {
    ($span:expr) => {
        ($span.to_owned().into(), None)
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
pub(crate) use help as note;

use crate::span::Span;

/// Writes a rich error report
///
/// This function should not be used in a loop as each call will
/// perform a light parse of the whole source code.
pub fn write_rich_error(
    error: &dyn RichError,
    file_name: &str,
    source_code: &str,
    w: impl std::io::Write,
) -> std::io::Result<()> {
    let cache = DummyCache::new(file_name, source_code);
    let report = build_report(error, source_code);
    report.write(cache, w)
}

fn build_report<'a>(err: &'a dyn RichError, src_code: &str) -> ariadne::Report<'a> {
    use ariadne::{Color, ColorGenerator, Fmt, Label, Report};

    let mut labels = err
        .labels()
        .into_iter()
        .map(|(s, t)| (s.to_chars_span(src_code).range(), t))
        .peekable();
    let offset = err
        .offset()
        .or_else(|| labels.peek().map(|l| l.0.start))
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
    r.add_labels(labels.enumerate().map(|(order, (span, text))| {
        let mut l = Label::new(span)
            .with_order(order as i32)
            .with_color(c.next());
        if let Some(text) = text {
            l = l.with_message(text);
        }
        l
    }));

    if let Some(help) = err.help() {
        r.set_help(help);
    }

    if let Some(note) = err.note() {
        r.set_note(note);
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
    fn labels(&self) -> Vec<(Span, Option<Cow<'static, str>>)> {
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

    fn note(&self) -> Option<Cow<'static, str>> {
        match self {
            CooklangError::Parser(e) => e.note(),
            CooklangError::Analysis(e) => e.note(),
            CooklangError::Io(_) => None,
            CooklangError::NoFilename { .. } => None,
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
    fn labels(&self) -> Vec<(Span, Option<Cow<'static, str>>)> {
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
