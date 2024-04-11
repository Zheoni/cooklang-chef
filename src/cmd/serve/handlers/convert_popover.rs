use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    Json,
};
use cooklang::ScaledQuantity;
use minijinja::context;

use crate::cmd::serve::{locale::UserLocale, S};

use super::mj_ok;

pub async fn convert_popover(
    UserLocale(t): UserLocale,
    State(state): State<S>,
    headers: HeaderMap,
    Json(quantity): Json<ScaledQuantity>,
) -> Response {
    let converter = state.parser.converter();

    let triggered_by = match headers.get("HX-Trigger").map(|v| v.to_str().ok()) {
        Some(id) => id,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    if quantity.unit().is_none() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let unit = match quantity.unit().unwrap().unit_info_or_parse(converter) {
        cooklang::UnitInfo::Known(unit) => unit,
        cooklang::UnitInfo::Unknown => return StatusCode::BAD_REQUEST.into_response(),
    };
    let conversions: Vec<_> = converter
        .best_units(unit.physical_quantity, None)
        .into_iter()
        .filter_map(|target| {
            let mut q = quantity.clone();
            q.convert(&target, converter).ok()?;
            Some(q)
        })
        .collect();

    let tmpl = mj_ok!(state
        .templates
        .get_template("components/convert_popover.html"));
    let html = mj_ok!(tmpl.render(context! { t, conversions, triggered_by }));
    Html(html).into_response()
}
