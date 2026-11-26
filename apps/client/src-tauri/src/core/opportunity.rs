use crate::models::job::WorkMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpportunityMetadata {
    pub location_cluster: &'static str,
    pub salary_min_idr: Option<i64>,
    pub salary_max_idr: Option<i64>,
    pub opportunity_type: &'static str,
}

pub fn extract_opportunity_metadata(
    title: &str,
    location: Option<&str>,
    work_mode: WorkMode,
    salary_text: Option<&str>,
    raw_description: &str,
) -> OpportunityMetadata {
    let (salary_min_idr, salary_max_idr) = extract_salary_idr(salary_text, raw_description);
    OpportunityMetadata {
        location_cluster: normalize_location_cluster(location, work_mode),
        salary_min_idr,
        salary_max_idr,
        opportunity_type: detect_opportunity_type(title, raw_description),
    }
}

pub fn normalize_location_cluster(location: Option<&str>, work_mode: WorkMode) -> &'static str {
    let loc = location.unwrap_or("").trim().to_lowercase();

    // 1. International remote or generic global markers
    let is_global_remote = [
        "remote globally",
        "worldwide",
        "global",
        "apac",
        "work from anywhere worldwide",
        "international remote",
    ]
    .iter()
    .any(|kw| loc.contains(kw));

    if is_global_remote {
        return "luar_negeri_remote";
    }

    // 2. Specific international hubs (before Indonesia remote fallback)
    if loc.contains("singapore") || loc.contains("singapura") || has_token(&loc, "sg") {
        return if work_mode == WorkMode::Remote || loc.contains("remote") {
            "luar_negeri_remote"
        } else {
            "singapura"
        };
    }
    if [
        "malaysia",
        "kuala lumpur",
        "penang",
        "johor",
        "selangor",
        "cyberjaya",
    ]
    .iter()
    .any(|kw| loc.contains(kw))
    {
        return if work_mode == WorkMode::Remote || loc.contains("remote") {
            "luar_negeri_remote"
        } else {
            "malaysia"
        };
    }
    if ["japan", "jepang", "tokyo", "osaka", "korea", "seoul", "taiwan", "taipei", "hong kong"]
        .iter()
        .any(|kw| loc.contains(kw))
    {
        return if work_mode == WorkMode::Remote || loc.contains("remote") {
            "luar_negeri_remote"
        } else {
            "jepang_asia_timur"
        };
    }
    if ["australia", "sydney", "melbourne", "brisbane", "perth", "new zealand", "auckland"]
        .iter()
        .any(|kw| loc.contains(kw))
    {
        return if work_mode == WorkMode::Remote || loc.contains("remote") {
            "luar_negeri_remote"
        } else {
            "australia_nz"
        };
    }
    if [
        "europe",
        "eropa",
        "london",
        "germany",
        "berlin",
        "netherlands",
        "amsterdam",
        "dubai",
        "united states",
        "united kingdom",
        "canada",
        "amerika",
    ]
    .iter()
    .any(|kw| loc.contains(kw))
        || has_token(&loc, "uk")
        || has_token(&loc, "usa")
        || has_token(&loc, "uae")
    {
        return if work_mode == WorkMode::Remote || loc.contains("remote") {
            "luar_negeri_remote"
        } else {
            "eropa_amerika"
        };
    }

    // 3. Indonesia Remote / WFH / WFA
    if work_mode == WorkMode::Remote
        || loc.contains("remote")
        || loc.contains("work from home")
        || loc.contains("work from anywhere")
        || loc.contains("anywhere in indonesia")
        || has_token(&loc, "wfh")
        || has_token(&loc, "wfa")
    {
        return "remote_indonesia";
    }

    // 4. DKI Jakarta
    if [
        "jakarta",
        "jaksel",
        "jakpus",
        "jakbar",
        "jaktim",
        "jakut",
        "sudirman",
        "kuningan",
        "senopati",
        "thamrin",
        "kemang",
        "pluit",
        "kelapa gading",
        "tb simatupang",
    ]
    .iter()
    .any(|kw| loc.contains(kw))
        || has_token(&loc, "scbd")
        || has_token(&loc, "pik")
    {
        return "jakarta";
    }

    // 5. Bodetabek (Bogor, Depok, Tangerang, Bekasi, Cikarang)
    if [
        "bodetabek",
        "jabodetabek",
        "bogor",
        "cibinong",
        "sentul",
        "depok",
        "tangerang",
        "tangsel",
        "alam sutera",
        "gading serpong",
        "karawaci",
        "bekasi",
        "cikarang",
        "tambun",
    ]
    .iter()
    .any(|kw| loc.contains(kw))
        || has_token(&loc, "bsd")
    {
        return "bodetabek";
    }

    // 6. Banten Lainnya
    if ["serang", "cilegon", "lebak", "pandeglang", "merak", "anyer", "banten"]
        .iter()
        .any(|kw| loc.contains(kw))
    {
        return "banten_lainnya";
    }

    // 7. Bandung Raya & Jawa Barat Lainnya
    if ["bandung", "cimahi", "sumedang", "jatinangor", "soreang", "lembang", "padalarang"]
        .iter()
        .any(|kw| loc.contains(kw))
    {
        return "bandung_raya";
    }
    if [
        "karawang",
        "purwakarta",
        "subang",
        "cirebon",
        "indramayu",
        "majalengka",
        "tasikmalaya",
        "garut",
        "sukabumi",
        "cianjur",
        "jawa barat",
        "west java",
    ]
    .iter()
    .any(|kw| loc.contains(kw))
    {
        return "jabar_lainnya";
    }

    // 8. DI Yogyakarta & Jawa Tengah
    if ["yogyakarta", "jogja", "sleman", "bantul", "kulon progo", "gunungkidul"]
        .iter()
        .any(|kw| loc.contains(kw))
        || has_token(&loc, "diy")
    {
        return "yogyakarta";
    }
    if [
        "semarang",
        "ungaran",
        "kendal",
        "salatiga",
        "demak",
        "kudus",
        "jepara",
        "pekalongan",
        "tegal",
        "batang",
        "brebes",
        "pemalang",
    ]
    .iter()
    .any(|kw| loc.contains(kw))
        || has_token(&loc, "pati")
    {
        return "semarang_raya";
    }
    if [
        "surakarta",
        "sukoharjo",
        "karanganyar",
        "boyolali",
        "klaten",
        "sragen",
        "wonogiri",
        "purwokerto",
        "banyumas",
        "cilacap",
        "magelang",
        "kebumen",
        "jawa tengah",
        "central java",
    ]
    .iter()
    .any(|kw| loc.contains(kw))
        || has_token(&loc, "solo")
    {
        return "solo_jateng";
    }

    // 9. Jawa Timur
    if ["surabaya", "sidoarjo", "gresik", "mojokerto", "lamongan", "bangkalan", "madura"]
        .iter()
        .any(|kw| loc.contains(kw))
    {
        return "surabaya_raya";
    }
    if [
        "malang",
        "pasuruan",
        "probolinggo",
        "kediri",
        "blitar",
        "tulungagung",
        "jember",
        "banyuwangi",
        "madiun",
        "nganjuk",
        "bojonegoro",
        "tuban",
        "jawa timur",
        "east java",
    ]
    .iter()
    .any(|kw| loc.contains(kw))
        || has_token(&loc, "batu")
    {
        return "malang_jatim";
    }

    // 10. Bali & Nusa Tenggara
    if [
        "denpasar",
        "badung",
        "kuta",
        "seminyak",
        "canggu",
        "ubud",
        "jimbaran",
        "nusa dua",
        "gianyar",
        "tabanan",
        "singaraja",
        "buleleng",
    ]
    .iter()
    .any(|kw| loc.contains(kw))
        || has_token(&loc, "bali")
    {
        return "bali";
    }
    if [
        "lombok",
        "mataram",
        "sumbawa",
        "bima",
        "nusa tenggara",
        "kupang",
        "flores",
        "labuan bajo",
        "maumere",
        "sumba",
    ]
    .iter()
    .any(|kw| loc.contains(kw))
        || has_token(&loc, "ntb")
        || has_token(&loc, "ntt")
        || has_token(&loc, "ende")
    {
        return "ntb_ntt";
    }

    // 11. Sumatera
    if [
        "medan",
        "deli serdang",
        "binjai",
        "pematangsiantar",
        "samosir",
        "sibolga",
        "sumatera utara",
        "sumut",
        "banda aceh",
        "lhokseumawe",
    ]
    .iter()
    .any(|kw| loc.contains(kw))
        || has_token(&loc, "aceh")
    {
        return "medan_sumut";
    }
    if ["batam", "bintan", "tanjung pinang", "karimun", "kepulauan riau", "kepri", "natuna"]
        .iter()
        .any(|kw| loc.contains(kw))
    {
        return "batam_kepri";
    }
    if [
        "pekanbaru",
        "dumai",
        "bengkalis",
        "siak",
        "padang",
        "bukittinggi",
        "sumatera barat",
        "sumbar",
        "jambi",
        "kerinci",
        "payakumbuh",
    ]
    .iter()
    .any(|kw| loc.contains(kw))
        || has_token(&loc, "riau")
    {
        return "pekanbaru_riau";
    }
    if [
        "palembang",
        "muara enim",
        "prabumulih",
        "sumatera selatan",
        "sumsel",
        "bandar lampung",
        "lampung",
        "bengkulu",
        "bangka",
        "belitung",
        "pangkalpinang",
        "sumatera",
        "sumatra",
    ]
    .iter()
    .any(|kw| loc.contains(kw))
    {
        return "palembang_sumbagsel";
    }

    // 12. Kalimantan
    if [
        "balikpapan",
        "samarinda",
        "nusantara",
        "penajam",
        "bontang",
        "sangatta",
        "kutai",
        "berau",
        "tarakan",
        "kalimantan timur",
        "kaltim",
        "kalimantan utara",
        "kaltara",
    ]
    .iter()
    .any(|kw| loc.contains(kw))
        || has_token(&loc, "ikn")
    {
        return "balikpapan_ikn";
    }
    if [
        "banjarmasin",
        "banjarbaru",
        "kalimantan selatan",
        "kalsel",
        "tabalong",
        "kotabaru",
        "palangkaraya",
        "sampit",
        "pangkalan bun",
        "kalimantan tengah",
        "kalteng",
    ]
    .iter()
    .any(|kw| loc.contains(kw))
    {
        return "banjarmasin_kalsel";
    }
    if ["pontianak", "singkawang", "ketapang", "sintang", "kalimantan barat", "kalbar", "kalimantan"]
        .iter()
        .any(|kw| loc.contains(kw))
    {
        return "pontianak_kalbar";
    }

    // 13. Sulawesi
    if [
        "makassar",
        "maros",
        "parepare",
        "sulawesi selatan",
        "sulsel",
        "morowali",
        "luwuk",
        "sulawesi tengah",
        "sulteng",
        "kendari",
        "kolaka",
        "sulawesi tenggara",
        "sultra",
        "mamuju",
        "sulawesi barat",
        "sulbar",
    ]
    .iter()
    .any(|kw| loc.contains(kw))
        || has_token(&loc, "gowa")
        || has_token(&loc, "palu")
    {
        return "makassar_sulsel";
    }
    if ["manado", "bitung", "tomohon", "minahasa", "sulawesi utara", "sulut", "gorontalo", "sulawesi"]
        .iter()
        .any(|kw| loc.contains(kw))
    {
        return "manado_sulut";
    }

    // 14. Maluku & Seluruh Papua
    if [
        "ambon",
        "maluku",
        "ternate",
        "halmahera",
        "weda",
        "malut",
        "jayapura",
        "sorong",
        "manokwari",
        "timika",
        "mimika",
        "nabire",
        "merauke",
        "papua",
    ]
    .iter()
    .any(|kw| loc.contains(kw))
        || has_token(&loc, "biak")
    {
        return "maluku_papua";
    }

    "indonesia_lainnya"
}

/// Expands legacy v0.4.0 cluster slugs into the v0.4.1 granular clusters so
/// saved watchlists and existing filters continue to match seamlessly.
pub fn expand_location_cluster_filter(clusters: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut push_unique = |slug: &str| {
        if !out.iter().any(|existing: &String| existing == slug) {
            out.push(slug.to_string());
        }
    };

    for raw in clusters {
        let slug = raw.trim().to_lowercase();
        if slug.is_empty() {
            continue;
        }
        push_unique(&slug);
        match slug.as_str() {
            "jabodetabek" => {
                push_unique("jakarta");
                push_unique("bodetabek");
            }
            "bandung_jabar" => {
                push_unique("bandung_raya");
                push_unique("jabar_lainnya");
            }
            "surabaya_jatim" => {
                push_unique("surabaya_raya");
                push_unique("malang_jatim");
            }
            "jogja_jateng" => {
                push_unique("yogyakarta");
                push_unique("semarang_raya");
                push_unique("solo_jateng");
            }
            "bali_ntt" => {
                push_unique("bali");
                push_unique("ntb_ntt");
            }
            "sumatera" => {
                push_unique("medan_sumut");
                push_unique("batam_kepri");
                push_unique("pekanbaru_riau");
                push_unique("palembang_sumbagsel");
            }
            "kalimantan" => {
                push_unique("balikpapan_ikn");
                push_unique("banjarmasin_kalsel");
                push_unique("pontianak_kalbar");
            }
            "sulawesi_timur" => {
                push_unique("makassar_sulsel");
                push_unique("manado_sulut");
                push_unique("maluku_papua");
            }
            "luar_negeri" => {
                push_unique("luar_negeri_remote");
                push_unique("singapura");
                push_unique("malaysia");
                push_unique("jepang_asia_timur");
                push_unique("australia_nz");
                push_unique("eropa_amerika");
            }
            _ => {}
        }
    }
    out
}

pub fn detect_opportunity_type(title: &str, raw_desc: &str) -> &'static str {
    let t = title.to_lowercase();
    let desc_head: String = raw_desc.chars().take(600).collect::<String>().to_lowercase();
    let combined = format!("{t} {desc_head}");

    if [
        "management trainee",
        "officer development program",
        "management development program",
        "future leader",
        "bumn muda",
        "graduate development program",
    ]
    .iter()
    .any(|kw| combined.contains(kw))
        || has_token(&t, "mt")
        || has_token(&t, "odp")
        || has_token(&t, "mdp")
    {
        return "mt_odp";
    }

    if [
        "magang",
        "internship",
        "msib",
        "magenta",
        "kerja praktik",
        "apprenticeship",
    ]
    .iter()
    .any(|kw| t.contains(kw) || has_token(&t, "intern") || has_token(&t, "pkl"))
    {
        return "internship";
    }

    if ["freelance", "paruh waktu", "part time", "part-time"]
        .iter()
        .any(|kw| t.contains(kw))
    {
        return "freelance";
    }

    if ["kontrak", "contract", "pkwt", "project based", "outsourc"]
        .iter()
        .any(|kw| t.contains(kw))
    {
        return "contract";
    }

    "full_time"
}

fn has_token(text: &str, token: &str) -> bool {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .any(|w| w == token)
}

pub fn extract_salary_idr(salary_text: Option<&str>, raw_desc: &str) -> (Option<i64>, Option<i64>) {
    if let Some(st) = salary_text.map(str::trim).filter(|s| !s.is_empty()) {
        let parsed = parse_salary_fragment(st);
        if parsed.0.is_some() {
            return parsed;
        }
    }

    for line in raw_desc.lines().take(60) {
        let lower = line.to_lowercase();
        if lower.contains("rp")
            || lower.contains("idr")
            || lower.contains("gaji")
            || lower.contains("salary")
            || lower.contains("juta")
            || lower.contains("jt")
            || lower.contains("usd")
        {
            let parsed = parse_salary_fragment(line);
            if parsed.0.is_some() {
                return parsed;
            }
        }
    }

    (None, None)
}

fn parse_salary_fragment(fragment: &str) -> (Option<i64>, Option<i64>) {
    let lower = fragment.to_lowercase();
    let is_usd = lower.contains("usd") || lower.contains('$');
    let is_yearly = lower.contains("/year") || lower.contains("per year") || lower.contains("/tahun");

    let bytes = lower.as_bytes();
    let mut i = 0usize;
    let mut vals: Vec<i64> = Vec::new();

    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len()
                && (bytes[i].is_ascii_digit() || bytes[i] == b'.' || bytes[i] == b',')
            {
                i += 1;
            }
            let num_str = &lower[start..i];
            let mut rest = lower[i..].trim_start();
            // Look ahead up to 8 chars for unit marker
            let unit_prefix: String = rest.chars().take(10).collect();

            if let Some(mut val) = parse_indonesian_or_std_number(num_str, &unit_prefix, is_usd) {
                if is_yearly {
                    val /= 12;
                }
                if is_usd && val > 0 && val < 30_000 {
                    val *= 16_000;
                }
                // Guard monthly IDR sanity bounds (1.5 jt .. 250 jt)
                if (1_500_000..=250_000_000).contains(&val) {
                    vals.push(val);
                }
            } else {
                // Handle "8 - 12 juta" where the unit appears on the second number
                rest = "";
                let _ = rest;
            }
        } else {
            i += 1;
        }
    }

    // Handle shorthand ranges like "8 - 12 juta" or "6-9 jt"
    if vals.len() == 1 {
        if let Some((a, b)) = try_parse_range_sharing_million(&lower) {
            return (Some(a), Some(b));
        }
    }

    if vals.is_empty() {
        return (None, None);
    }

    vals.sort_unstable();
    let min_v = vals[0];
    let max_v = if vals.len() > 1 {
        *vals.last().unwrap_or(&min_v)
    } else {
        min_v
    };
    (Some(min_v), Some(max_v))
}

fn try_parse_range_sharing_million(lower: &str) -> Option<(i64, i64)> {
    let has_jt = lower.contains("juta") || lower.contains("jt") || lower.contains("mio");
    if !has_jt {
        return None;
    }
    let mut nums = Vec::new();
    let mut cur = String::new();
    for ch in lower.chars() {
        if ch.is_ascii_digit() || ch == '.' || ch == ',' {
            cur.push(ch);
        } else if !cur.is_empty() {
            let norm = cur.replace(',', ".");
            if let Ok(v) = norm.parse::<f64>() {
                if (1.5..=250.0).contains(&v) {
                    nums.push((v * 1_000_000.0).round() as i64);
                }
            }
            cur.clear();
        }
    }
    if !cur.is_empty() {
        let norm = cur.replace(',', ".");
        if let Ok(v) = norm.parse::<f64>() {
            if (1.5..=250.0).contains(&v) {
                nums.push((v * 1_000_000.0).round() as i64);
            }
        }
    }
    if nums.len() >= 2 {
        nums.sort_unstable();
        return Some((nums[0], *nums.last().unwrap_or(&nums[0])));
    }
    None
}

fn parse_indonesian_or_std_number(num_str: &str, unit_after: &str, is_usd: bool) -> Option<i64> {
    let clean = num_str.trim_matches(|c| c == '.' || c == ',');
    if clean.is_empty() {
        return None;
    }

    let is_jt = unit_after.starts_with("jt")
        || unit_after.starts_with("juta")
        || unit_after.starts_with("mio")
        || unit_after.starts_with("m ");
    let is_rb = unit_after.starts_with("rb") || unit_after.starts_with("ribu");

    if is_jt {
        let norm = clean.replace(',', ".");
        let v: f64 = norm.parse().ok()?;
        return Some((v * 1_000_000.0).round() as i64);
    }
    if is_rb {
        let norm = clean.replace(',', ".");
        let v: f64 = norm.parse().ok()?;
        return Some((v * 1_000.0).round() as i64);
    }

    let digits: String = clean.chars().filter(|c| c.is_ascii_digit()).collect();
    if is_usd {
        return digits.parse::<i64>().ok();
    }
    // Plain Indonesian formatted numbers like 8.000.000 or 12,000,000
    if digits.len() >= 7 {
        return digits.parse::<i64>().ok();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_indonesian_and_remote_location_clusters() {
        assert_eq!(
            normalize_location_cluster(Some("Jakarta Selatan, DKI Jakarta"), WorkMode::Hybrid),
            "jakarta"
        );
        assert_eq!(
            normalize_location_cluster(Some("Tangerang Selatan / BSD"), WorkMode::Onsite),
            "bodetabek"
        );
        assert_eq!(
            normalize_location_cluster(Some("Bandung, Jawa Barat"), WorkMode::Onsite),
            "bandung_raya"
        );
        assert_eq!(
            normalize_location_cluster(Some("Surabaya"), WorkMode::Onsite),
            "surabaya_raya"
        );
        assert_eq!(
            normalize_location_cluster(Some("Sleman, DI Yogyakarta"), WorkMode::Onsite),
            "yogyakarta"
        );
        assert_eq!(
            normalize_location_cluster(Some("Solo / Surakarta"), WorkMode::Onsite),
            "solo_jateng"
        );
        assert_eq!(
            normalize_location_cluster(Some("Batam, Kepulauan Riau"), WorkMode::Onsite),
            "batam_kepri"
        );
        assert_eq!(
            normalize_location_cluster(Some("Jakarta"), WorkMode::Remote),
            "remote_indonesia"
        );
        assert_eq!(
            normalize_location_cluster(Some("Singapore (Remote)"), WorkMode::Remote),
            "luar_negeri_remote"
        );
        assert_eq!(
            normalize_location_cluster(Some("Singapore CBD"), WorkMode::Onsite),
            "singapura"
        );
        assert_eq!(
            normalize_location_cluster(Some("Balikpapan, Kalimantan Timur"), WorkMode::Onsite),
            "balikpapan_ikn"
        );
        assert_eq!(
            normalize_location_cluster(Some("Jayapura, Papua"), WorkMode::Onsite),
            "maluku_papua"
        );
        let expanded = expand_location_cluster_filter(&["jabodetabek".into()]);
        assert!(expanded.contains(&"jakarta".to_string()));
        assert!(expanded.contains(&"bodetabek".to_string()));
    }

    #[test]
    fn extracts_indonesian_salary_formats_safely() {
        assert_eq!(
            extract_salary_idr(Some("Rp 8.000.000 - Rp 12.500.000 / bulan"), ""),
            (Some(8_000_000), Some(12_500_000))
        );
        assert_eq!(
            extract_salary_idr(Some("IDR 6,5 jt - 9 jt"), ""),
            (Some(6_500_000), Some(9_000_000))
        );
        assert_eq!(
            extract_salary_idr(None, "Benefit:\n- Gaji 7 - 10 juta per bulan\n- BPJS"),
            (Some(7_000_000), Some(10_000_000))
        );
        assert_eq!(
            extract_salary_idr(None, "Minimal 3 tahun pengalaman sejak 2022"),
            (None, None)
        );
    }

    #[test]
    fn detects_opportunity_types() {
        assert_eq!(
            detect_opportunity_type("Management Trainee Commercial 2026", ""),
            "mt_odp"
        );
        assert_eq!(
            detect_opportunity_type("ODP Wholesale Banking Mandiri", ""),
            "mt_odp"
        );
        assert_eq!(
            detect_opportunity_type("Software Engineer Intern (MSIB)", ""),
            "internship"
        );
        assert_eq!(
            detect_opportunity_type("Admin Finance (Kontrak PKWT 1 Tahun)", ""),
            "contract"
        );
        assert_eq!(
            detect_opportunity_type("Backend Developer Golang", ""),
            "full_time"
        );
    }
}
