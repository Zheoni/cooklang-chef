use miette::Diagnostic;
use thiserror::Error;

pub type CookResult<T> = Result<T, CooklangError>;

#[derive(Debug, Error, Diagnostic)]
pub enum CooklangError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Parser(#[from] crate::parser::ParserReport),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Analysis(#[from] crate::analysis::AnalysisReport),
    #[error(transparent)]
    #[diagnostic(code(cooklang::io))]
    Io(#[from] std::io::Error),
    #[error("No file name in path: '{path}'")]
    #[diagnostic(
        code(cooklang::no_file_name),
        help("The recipe name is needed and comes from the file name")
    )]
    NoFilename { path: std::path::PathBuf },
}

#[derive(Debug, Error, Diagnostic)]
#[error(transparent)]
#[diagnostic(transparent)]
pub enum CooklangWarning {
    Parser(#[from] crate::parser::ParserWarning),
    Analysis(#[from] crate::analysis::AnalysisWarning),
}

pub fn print_warnings(input: &str, warnings: &[CooklangWarning]) {
    if warnings.is_empty() {
        return;
    }

    #[derive(Debug, thiserror::Error, miette::Diagnostic)]
    #[error("Warning{}", if self.warnings.len() > 1 { "s" } else { "" })]
    #[diagnostic(severity(warning))]
    struct Report<'b> {
        #[source_code]
        input: &'b str,

        #[related]
        warnings: &'b [CooklangWarning],
    }

    let handler = miette::GraphicalReportHandler::new();
    let mut s = String::new();
    let report = Report { input, warnings };
    handler.render_report(&mut s, &report).unwrap();
    eprintln!("{s}");
}
