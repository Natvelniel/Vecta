use crate::models::DocumentSections;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveSection {
    General,
    Requirements,
    Responsibilities,
    NiceToHave,
    AboutCompany,
}

fn detect_heading(line: &str) -> Option<ActiveSection> {
    let trimmed = line
        .trim()
        .trim_end_matches(':')
        .trim_matches(|c: char| c == '#' || c == '*' || c == '-' || c == '•')
        .trim();
    if trimmed.is_empty() || trimmed.len() > 64 {
        return None;
    }
    let lower = trimmed.to_lowercase();

    const NICE_HEADERS: &[&str] = &[
        "nice to have",
        "nilai tambah",
        "preferred",
        "preferred qualifications",
        "plus point",
        "bonus",
        "diutamakan",
        "keunggulan tambahan",
    ];
    if NICE_HEADERS.iter().any(|h| lower == *h || lower.starts_with(&format!("{h} "))) {
        return Some(ActiveSection::NiceToHave);
    }

    const REQ_HEADERS: &[&str] = &[
        "kualifikasi",
        "persyaratan",
        "syarat",
        "requirements",
        "requirement",
        "qualifications",
        "qualification",
        "minimum qualifications",
        "what you bring",
        "who you are",
        "skills required",
        "kriteria",
    ];
    if REQ_HEADERS.iter().any(|h| lower == *h || lower.starts_with(&format!("{h} "))) {
        return Some(ActiveSection::Requirements);
    }

    const RESP_HEADERS: &[&str] = &[
        "tanggung jawab",
        "deskripsi pekerjaan",
        "uraian tugas",
        "tugas dan tanggung jawab",
        "responsibilities",
        "responsibility",
        "job description",
        "what you will do",
        "key responsibilities",
        "peran dan tanggung jawab",
    ];
    if RESP_HEADERS.iter().any(|h| lower == *h || lower.starts_with(&format!("{h} "))) {
        return Some(ActiveSection::Responsibilities);
    }

    const ABOUT_HEADERS: &[&str] = &[
        "tentang perusahaan",
        "profil perusahaan",
        "tentang kami",
        "about us",
        "about the company",
        "company overview",
        "company profile",
        "who we are",
        "benefit",
        "benefits",
        "fasilitas",
    ];
    if ABOUT_HEADERS.iter().any(|h| lower == *h || lower.starts_with(&format!("{h} "))) {
        return Some(ActiveSection::AboutCompany);
    }

    None
}

/// Split a raw Indonesian or English job description into semantic sections so company overview
/// buzzwords do not pollute required job skills.
pub fn segment_job_description(raw: &str) -> DocumentSections {
    let mut sections = DocumentSections::default();
    let mut active = ActiveSection::General;

    for line in raw.lines() {
        if let Some(next_sec) = detect_heading(line) {
            active = next_sec;
            continue;
        }
        let target = match active {
            ActiveSection::General => &mut sections.general,
            ActiveSection::Requirements => &mut sections.requirements,
            ActiveSection::Responsibilities => &mut sections.responsibilities,
            ActiveSection::NiceToHave => &mut sections.nice_to_have,
            ActiveSection::AboutCompany => &mut sections.about_company,
        };
        if !target.is_empty() {
            target.push('\n');
        }
        target.push_str(line);
    }

    sections
}
