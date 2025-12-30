mod css;
mod html;
mod js;

use crate::application::use_cases::ReportData;

/// インタラクティブレポートを生成する
pub fn generate_interactive_report(data: &ReportData) -> String {
    let json_data = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());

    let template = html::get_html_template();

    template
        .replace("{date}", &data.generated_at.format("%Y-%m-%d").to_string())
        .replace("{generated_at}", &data.generated_at.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .replace("{total_issues}", &data.total_issues.to_string())
        .replace("{css}", css::get_css())
        .replace("{json_data}", &json_data)
        .replace("{js}", js::get_js())
}
