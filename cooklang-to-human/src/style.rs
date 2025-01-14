use anstyle::Style;

macro_rules! map_style_type {
    (Style) => {
        yansi::Style
    };
    ($other:ty) => {
        $other
    };
}

macro_rules! map_style_func {
    ($s:ident, $name:ident, Style) => {
        anstyle_yansi::to_yansi_style($s.$name)
    };
    ($s:ident, $name:ident, $type:ty) => {
        $s.$name
    };
}

macro_rules! generate_styles_struct {
    ($($v:vis $field_name:ident : $field_type:tt = $default:expr),+ $(,)?) => {
        #[derive(Debug, Clone)]
        #[non_exhaustive]
        pub struct CookStyles { $($v $field_name: $field_type),+ }

        #[derive(Debug, Clone)]
        pub(crate) struct OwoStyles { $(pub $field_name: map_style_type!($field_type)),* }

        impl From<CookStyles> for OwoStyles {
            fn from(s: CookStyles) -> OwoStyles {
                OwoStyles {
                    $($field_name: map_style_func!(s, $field_name, $field_type)),+
                }
            }
        }

        impl CookStyles {
            pub const fn default_styles() -> Self {
                Self {
                    $($field_name: $default),+
                }
            }
        }
    };
}

macro_rules! color {
    ($color:ident) => {
        Some(anstyle::Color::Ansi(anstyle::AnsiColor::$color))
    };
}

// macro magic to generate 2 struct CookStyles and OwoStyles same fields, but
// when Style is used here, CookStyles will have anstyle::Style and OwoStyles
// owo_colors::Style for internal use. Also, OwoStyles impl From<CookStyles>

generate_styles_struct! {
    pub title: Style             = Style::new().fg_color(color!(White)).bg_color(color!(Magenta)).bold(),
    pub meta_key: Style          = Style::new().fg_color(color!(BrightGreen)).bold(),
    pub selected_servings: Style = Style::new().fg_color(color!(Yellow)).bold(),
    pub ingredient: Style        = Style::new().fg_color(color!(Green)),
    pub cookware: Style          = Style::new().fg_color(color!(Yellow)),
    pub timer: Style             = Style::new().fg_color(color!(Cyan)),
    pub inline_quantity: Style   = Style::new().fg_color(color!(BrightRed)),
    pub opt_marker: Style        = Style::new().fg_color(color!(BrightCyan)).italic(),
    pub intermediate_ref: Style  = Style::new().fg_color(color!(BrightYellow)).italic(),
    pub section_name: Style      = Style::new().bold().underline(),
    pub step_igr_quantity: Style = Style::new().dimmed(),
}

static STYLE: std::sync::OnceLock<OwoStyles> = std::sync::OnceLock::new();

/// Set custom styles
///
/// Returns true if the styles were set.
///
/// This can only be called once and before any formatting is done, otherwise
/// it will return false.
pub fn set_styles(styles: CookStyles) -> bool {
    STYLE.set(styles.into()).is_ok()
}

#[inline]
pub(crate) fn styles() -> &'static OwoStyles {
    STYLE.get_or_init(|| CookStyles::default_styles().into())
}
