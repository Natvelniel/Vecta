pub mod dealls;
pub mod jobstreet;
pub mod kalibrr;
pub mod karirhub;
pub mod linkedin;
pub mod remotive;

/// Formats non-empty named sections into a clean, consistent readable document for UI and Neural v2.
pub fn compose_structured_description(sections: &[(&str, String)]) -> String {
    let mut blocks = Vec::new();
    for (heading, body) in sections {
        let trimmed = body.trim();
        if trimmed.is_empty() {
            continue;
        }
        if heading.is_empty() {
            blocks.push(trimmed.to_string());
        } else {
            blocks.push(format!("{heading}:\n{trimmed}"));
        }
    }
    blocks.join("\n\n")
}
