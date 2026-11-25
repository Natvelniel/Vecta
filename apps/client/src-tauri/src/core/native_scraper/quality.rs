use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct JobRedFlag {
    pub code: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobQualityAssessment {
    pub quality_score: f64,
    pub red_flags: Vec<JobRedFlag>,
}

/// Evaluate job listing safety & transparency locally (0 Gemini tokens).
/// Flags common Indonesian recruitment scams, suspicious private contacts, and sparse listings.
pub fn assess_job_quality(
    title: &str,
    company: &str,
    raw_description: &str,
    salary_text: Option<&str>,
    url: Option<&str>,
) -> JobQualityAssessment {
    let lower = format!(
        "{}\n{}\n{}",
        title.to_lowercase(),
        company.to_lowercase(),
        raw_description.to_lowercase()
    );
    let mut flags = Vec::new();
    let mut score = 1.0f64;

    const FEE_PATTERNS: &[&str] = &[
        "biaya administrasi",
        "biaya pendaftaran",
        "biaya akomodasi",
        "biaya travel",
        "biaya tiket",
        "biaya tes",
        "biaya seragam",
        "biaya training",
        "transfer biaya",
        "rekening pribadi",
        "titip ijazah",
        "tahan ijazah",
        "penahanan ijazah",
    ];
    if FEE_PATTERNS.iter().any(|p| lower.contains(p)) {
        score -= 0.45;
        flags.push(JobRedFlag {
            code: "SUSPICIOUS_FEE_OR_DIPLOMA".into(),
            severity: "high".into(),
            message: "Terdeteksi indikasi permintaan biaya rekrutmen/travel atau penahanan ijazah. Verifikasi langsung ke situs resmi perusahaan.".into(),
        });
    }

    const UNREALISTIC_PATTERNS: &[&str] = &[
        "tanpa pengalaman gaji 15",
        "tanpa pengalaman gaji 20",
        "penghasilan 500rb per hari",
        "penghasilan jutaan per hari",
        "kerja ketik dari rumah",
        "like dan subscribe dibayar",
        "kirim lamaran via wa pribadi",
    ];
    if UNREALISTIC_PATTERNS.iter().any(|p| lower.contains(p)) {
        score -= 0.40;
        flags.push(JobRedFlag {
            code: "UNREALISTIC_OFFER".into(),
            severity: "high".into(),
            message: "Tawaran penghasilan atau instruksi lamaran menyerupai pola lowongan tidak resmi.".into(),
        });
    }

    let has_wa_contact = lower.contains("hubungi wa")
        || lower.contains("kirim ke wa")
        || lower.contains("whatsapp 08")
        || lower.contains("wa.me/");
    let has_official_domain = url
        .map(|u| {
            let ul = u.to_lowercase();
            ul.contains("jobstreet")
                || ul.contains("linkedin")
                || ul.contains("kalibrr")
                || ul.contains("dealls")
                || ul.contains("indeed")
                || ul.contains("glints")
        })
        .unwrap_or(false);
    if has_wa_contact && !has_official_domain {
        score -= 0.15;
        flags.push(JobRedFlag {
            code: "PRIVATE_WHATSAPP_CONTACT".into(),
            severity: "medium".into(),
            message: "Mengarahkan kontak langsung ke WhatsApp pribadi tanpa tautan portal resmi.".into(),
        });
    }

    if company.trim().eq_ignore_ascii_case("unknown company")
        || company.trim().eq_ignore_ascii_case("confidential")
        || company.trim().eq_ignore_ascii_case("perusahaan dirahasiakan")
    {
        score -= 0.10;
        flags.push(JobRedFlag {
            code: "HIDDEN_COMPANY".into(),
            severity: "low".into(),
            message: "Nama perusahaan tidak dicantumkan secara terbuka.".into(),
        });
    }

    if raw_description.trim().chars().count() < 120 {
        score -= 0.10;
        flags.push(JobRedFlag {
            code: "SPARSE_DESCRIPTION".into(),
            severity: "low".into(),
            message: "Deskripsi lowongan sangat singkat sehingga detail kualifikasi terbatas.".into(),
        });
    }

    if salary_text.map(|s| !s.trim().is_empty()).unwrap_or(false) {
        score = (score + 0.05).min(1.0);
    }

    JobQualityAssessment {
        quality_score: score.clamp(0.05, 1.0),
        red_flags: flags,
    }
}

fn canonicalize_key_part(input: &str) -> String {
    let lower = input.to_lowercase();
    let cleaned: String = lower
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    cleaned
        .split_whitespace()
        .filter(|w| !matches!(*w, "pt" | "cv" | "tbk" | "indonesia" | "ltd" | "inc" | "corp"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn token_shingles(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 3)
        .take(120)
        .map(str::to_string)
        .collect()
}

fn jaccard_similarity(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count();
    let union = a.union(b).count();
    inter as f64 / union.max(1) as f64
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct PortalLinkRef {
    pub source: String,
    pub url: Option<String>,
}

/// Merge duplicate jobs fetched across multiple portals (e.g., same PT + title on JobStreet & LinkedIn)
/// while keeping the richest description and recording alternate portal links.
pub fn deduplicate_cross_portal<T>(
    items: Vec<T>,
    key_fn: impl Fn(&T) -> (String, String, String, String, Option<String>, String),
) -> Vec<(T, Vec<PortalLinkRef>)> {
    let mut kept: Vec<(T, Vec<PortalLinkRef>, HashSet<String>)> = Vec::new();
    let mut bucket: HashMap<(String, String), Vec<usize>> = HashMap::new();

    for item in items {
        let (company, title, _location, source, url, desc) = key_fn(&item);
        let key = (canonicalize_key_part(&company), canonicalize_key_part(&title));
        let shingles = token_shingles(&desc);

        let mut merged_into: Option<usize> = None;
        if let Some(indices) = bucket.get(&key) {
            for &idx in indices {
                let (_, existing_links, existing_shingles) = &kept[idx];
                let same_source = existing_links.iter().any(|l| l.source == source);
                if same_source || jaccard_similarity(existing_shingles, &shingles) >= 0.45 {
                    merged_into = Some(idx);
                    break;
                }
            }
        }

        if let Some(idx) = merged_into {
            let (existing_item, existing_links, existing_shingles) = &mut kept[idx];
            if !existing_links.iter().any(|l| l.source == source) {
                existing_links.push(PortalLinkRef { source, url });
            }
            let existing_desc_len = key_fn(existing_item).5.len();
            if desc.len() > existing_desc_len {
                *existing_item = item;
                *existing_shingles = shingles;
            }
        } else {
            let new_idx = kept.len();
            bucket.entry(key).or_default().push(new_idx);
            kept.push((item, vec![PortalLinkRef { source, url }], shingles));
        }
    }

    kept.into_iter()
        .map(|(item, mut links, _)| {
            let primary_source = key_fn(&item).3;
            links.retain(|l| l.source != primary_source);
            (item, links)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn red_flag_detector_catches_indonesian_recruitment_scam() {
        let bad = assess_job_quality(
            "Admin Gudang Gaji 15 Juta",
            "Confidential",
            "Tanpa pengalaman langsung kerja! Wajib transfer biaya administrasi dan biaya travel ke rekening pribadi HRD serta titip ijazah.",
            None,
            None,
        );
        assert!(bad.quality_score < 0.6);
        assert!(bad
            .red_flags
            .iter()
            .any(|f| f.code == "SUSPICIOUS_FEE_OR_DIPLOMA"));

        let good = assess_job_quality(
            "Staff Akuntansi & Pajak",
            "PT Nusantara Sejahtera",
            "Kualifikasi: S1 Akuntansi, menguasai Brevet A/B, penyusunan laporan keuangan sesuai PSAK, e-Faktur, Accurate Online, dan rekonsiliasi bank harian di kantor pusat Bandung.",
            Some("Rp 7.000.000 - Rp 9.000.000"),
            Some("https://id.jobstreet.com/job/12345"),
        );
        assert!(good.quality_score >= 0.95);
        assert!(good.red_flags.is_empty());
    }

    #[test]
    fn cross_portal_deduplication_merges_same_job_and_preserves_alternate_portal() {
        #[derive(Clone)]
        struct Stub {
            company: String,
            title: String,
            source: String,
            url: Option<String>,
            desc: String,
        }
        let jobs = vec![
            Stub {
                company: "PT Nusantara Teknologi Tbk".into(),
                title: "Backend Developer PHP".into(),
                source: "jobstreet".into(),
                url: Some("https://id.jobstreet.com/job/1".into()),
                desc: "Membangun API PHP Laravel MySQL di Bandung untuk layanan logistik".into(),
            },
            Stub {
                company: "Nusantara Teknologi".into(),
                title: "Backend Developer PHP".into(),
                source: "linkedin".into(),
                url: Some("https://www.linkedin.com/jobs/view/2".into()),
                desc: "Membangun API PHP Laravel MySQL di Bandung untuk layanan logistik dan integrasi pembayaran".into(),
            },
        ];

        let merged = deduplicate_cross_portal(jobs, |j| {
            (
                j.company.clone(),
                j.title.clone(),
                "Bandung".into(),
                j.source.clone(),
                j.url.clone(),
                j.desc.clone(),
            )
        });
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].0.source, "linkedin");
        assert_eq!(merged[0].1.len(), 1);
        assert_eq!(merged[0].1[0].source, "jobstreet");
    }
}
