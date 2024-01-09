use axum::{
    extract::State,
    response::{Html, IntoResponse, Response},
};
use minijinja::context;

use crate::serve::{locale::UserLocale, Assets, S};

use super::mj_ok;

pub async fn about(UserLocale(t): UserLocale, State(state): State<S>) -> Response {
    let tmpl = mj_ok!(state.templates.get_template("about.html"));

    let font_licenses_file = Assets::get("fonts/LICENSES").expect("can't find font licenses");
    let font_licenses =
        std::str::from_utf8(font_licenses_file.data.as_ref()).expect("font licenses not utf8");
    let vendor_licenses_file = Assets::get("vendor/LICENSES").expect("can't find font licenses");
    let vendor_licenses =
        std::str::from_utf8(vendor_licenses_file.data.as_ref()).expect("vendor licenses not utf8");

    let res = tmpl
        .render(context! { t, FONT_LICENSES => font_licenses, VENDOR_LICENSES => vendor_licenses });
    let content = mj_ok!(res);
    Html(content).into_response()
}
