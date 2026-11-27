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

fn truncate_before_company_bio(raw_desc: &str) -> String {
    let mut kept = Vec::new();
    for line in raw_desc.lines() {
        let lower = line
            .trim()
            .trim_end_matches(':')
            .trim()
            .to_lowercase();
        if matches!(
            lower.as_str(),
            "tentang perusahaan"
                | "about company"
                | "about the company"
                | "about us"
                | "profil perusahaan"
                | "mengenai perusahaan"
                | "tentang kami"
                | "company profile"
                | "company overview"
        ) {
            break;
        }
        kept.push(line);
    }
    kept.join("\n")
}

fn has_currency_prefix(lower: &str) -> bool {
    if lower.contains('$') {
        return true;
    }
    let bytes = lower.as_bytes();
    for (idx, _) in lower.match_indices("rp") {
        let left_ok = idx == 0 || !bytes[idx - 1].is_ascii_alphanumeric();
        let after = idx + 2;
        let right_ok = after < bytes.len()
            && (bytes[after].is_ascii_digit()
                || bytes[after] == b' '
                || bytes[after] == b'.'
                || bytes[after] == b':');
        if left_ok && right_ok {
            return true;
        }
    }
    for (idx, _) in lower.match_indices("idr") {
        let left_ok = idx == 0 || !bytes[idx - 1].is_ascii_alphanumeric();
        let after = idx + 3;
        let right_ok = after == bytes.len()
            || bytes[after].is_ascii_digit()
            || !bytes[after].is_ascii_alphabetic();
        if left_ok && right_ok {
            return true;
        }
    }
    has_token(lower, "usd") || has_token(lower, "sgd")
}

fn has_salary_context_keyword(lower: &str) -> bool {
    if has_currency_prefix(lower) {
        return true;
    }
    [
        "gaji",
        "salary",
        "upah",
        "thp",
        "kompensasi",
        "penghasilan",
        "remunerasi",
        "honor",
        "tunjangan",
        "bayaran",
    ]
    .iter()
    .any(|kw| has_token(lower, kw))
        || lower.contains("take home pay")
        || lower.contains("uang saku")
}

fn is_followed_by_non_money_noun(after_unit: &str) -> bool {
    let next_word = after_unit
        .split(|c: char| !c.is_ascii_alphabetic())
        .find(|w| !w.is_empty())
        .unwrap_or("");
    matches!(
        next_word,
        "pengguna"
            | "user"
            | "users"
            | "siswa"
            | "pelajar"
            | "mahasiswa"
            | "pelanggan"
            | "customer"
            | "customers"
            | "unduhan"
            | "download"
            | "downloads"
            | "transaksi"
            | "follower"
            | "followers"
            | "penonton"
            | "penduduk"
            | "orang"
            | "guru"
            | "karyawan"
            | "pegawai"
            | "toko"
            | "outlet"
            | "cabang"
            | "bidang"
            | "anggota"
            | "member"
            | "pekerja"
            | "kali"
            | "tahun"
    )
}

pub fn extract_salary_idr(salary_text: Option<&str>, raw_desc: &str) -> (Option<i64>, Option<i64>) {
    if let Some(st) = salary_text.map(str::trim).filter(|s| !s.is_empty()) {
        let lower = st.to_lowercase();
        let parsed = parse_salary_fragment(&lower, true);
        if parsed.0.is_some() {
            return parsed;
        }
    }

    let cleaned_desc = truncate_before_company_bio(raw_desc);
    for line in cleaned_desc.lines().take(60) {
        let lower = line.to_lowercase();
        if !has_salary_context_keyword(&lower) {
            continue;
        }
        let parsed = parse_salary_fragment(&lower, false);
        if parsed.0.is_some() {
            return parsed;
        }
    }

    (None, None)
}

fn parse_salary_fragment(lower: &str, trusted_salary_field: bool) -> (Option<i64>, Option<i64>) {
    let is_usd = has_token(lower, "usd") || lower.contains('$');
    let is_yearly = lower.contains("/year")
        || lower.contains("per year")
        || lower.contains("/tahun")
        || lower.contains("per tahun");
    let has_money_signal = trusted_salary_field || has_salary_context_keyword(lower);
    if !has_money_signal {
        return (None, None);
    }

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
            let rest = lower[i..].trim_start();
            let unit_prefix: String = rest.chars().take(12).collect();

            if let Some((mut val, consumed_unit_len)) =
                parse_indonesian_or_std_number(num_str, &unit_prefix, is_usd)
            {
                let after_unit = rest.get(consumed_unit_len..).unwrap_or("");
                if consumed_unit_len > 0 && is_followed_by_non_money_noun(after_unit) {
                    continue;
                }
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
            }
        } else {
            i += 1;
        }
    }

    // Handle adjacent shorthand ranges like "8 - 12 juta" or "6-9 jt"
    if vals.len() == 1 {
        if let Some((a, b)) = try_parse_range_sharing_million(lower) {
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

fn parse_float_token(s: &str) -> Option<f64> {
    let clean = s.trim_matches(|c: char| c == '.' || c == ',');
    if clean.is_empty() {
        return None;
    }
    clean.replace(',', ".").parse::<f64>().ok()
}

fn try_parse_range_sharing_million(lower: &str) -> Option<(i64, i64)> {
    // Only accept directly connected range: <num1> (-|–|hingga|sampai|to|s/d|sd) <num2> (juta|jt|mio)
    let bytes = lower.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let n1_start = i;
        while i < bytes.len()
            && (bytes[i].is_ascii_digit() || bytes[i] == b'.' || bytes[i] == b',')
        {
            i += 1;
        }
        let Some(v1) = parse_float_token(&lower[n1_start..i]) else {
            continue;
        };
        if !(1.5..=250.0).contains(&v1) {
            continue;
        }

        let after_n1 = lower[i..].trim_start();
        let rest_after_conn = after_n1
            .strip_prefix('-')
            .or_else(|| after_n1.strip_prefix('–'))
            .or_else(|| after_n1.strip_prefix('~'))
            .or_else(|| after_n1.strip_prefix("s/d"))
            .or_else(|| after_n1.strip_prefix("sd "))
            .or_else(|| after_n1.strip_prefix("hingga "))
            .or_else(|| after_n1.strip_prefix("sampai "))
            .or_else(|| after_n1.strip_prefix("to "));

        let Some(after_conn_raw) = rest_after_conn else {
            continue;
        };
        let after_conn = after_conn_raw.trim_start();
        let n2_len = after_conn
            .bytes()
            .take_while(|b| b.is_ascii_digit() || *b == b'.' || *b == b',')
            .count();
        if n2_len == 0 {
            continue;
        }
        let Some(v2) = parse_float_token(&after_conn[..n2_len]) else {
            continue;
        };
        if !(1.5..=250.0).contains(&v2) || v2 < v1 {
            continue;
        }

        let after_n2 = after_conn[n2_len..].trim_start();
        let unit_len = if after_n2.starts_with("juta") {
            4
        } else if after_n2.starts_with("mio") {
            3
        } else if after_n2.starts_with("jt") {
            2
        } else {
            0
        };
        if unit_len == 0 {
            continue;
        }
        let after_unit = &after_n2[unit_len..];
        if is_followed_by_non_money_noun(after_unit) {
            continue;
        }
        let a = (v1 * 1_000_000.0).round() as i64;
        let b = (v2 * 1_000_000.0).round() as i64;
        return Some((a.min(b), a.max(b)));
    }
    None
}

fn parse_indonesian_or_std_number(
    num_str: &str,
    unit_after: &str,
    is_usd: bool,
) -> Option<(i64, usize)> {
    let clean = num_str.trim_matches(|c| c == '.' || c == ',');
    if clean.is_empty() {
        return None;
    }

    let unit_len = if unit_after.starts_with("juta") {
        4
    } else if unit_after.starts_with("mio") {
        3
    } else if unit_after.starts_with("jt") {
        2
    } else if unit_after.starts_with("m ") {
        1
    } else {
        0
    };
    if unit_len > 0 {
        let norm = clean.replace(',', ".");
        let v: f64 = norm.parse().ok()?;
        return Some(((v * 1_000_000.0).round() as i64, unit_len));
    }

    let rb_len = if unit_after.starts_with("ribu") {
        4
    } else if unit_after.starts_with("rb") {
        2
    } else {
        0
    };
    if rb_len > 0 {
        let norm = clean.replace(',', ".");
        let v: f64 = norm.parse().ok()?;
        return Some(((v * 1_000.0).round() as i64, rb_len));
    }

    let digits: String = clean.chars().filter(|c| c.is_ascii_digit()).collect();
    if is_usd {
        return digits.parse::<i64>().ok().map(|v| (v, 0));
    }
    // Plain Indonesian formatted numbers like 8.000.000 or 12,000,000
    if digits.len() >= 7 {
        return digits.parse::<i64>().ok().map(|v| (v, 0));
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
        // Regression guard against false positives in Kalibrr/Ruangguru company bios and Indonesian words ("berperan", "berkelanjutan")
        let ruangguru_desc = "\
Deskripsi Pekerjaan\nMengajar siswa UTBK dan berperan aktif sebagai mentor kelas berkelanjutan.\n\
Tentang Perusahaan\n\
Ruangguru merupakan perusahaan teknologi terbesar dan terlengkap di Indonesia yang berfokus pada layanan berbasis pendidikan dan telah memiliki lebih dari 13 juta pengguna serta telah mengelola lebih dari 500.000 guru yang menawarkan jasa di lebih dari 100 bidang pelajaran.";
        assert_eq!(extract_salary_idr(None, ruangguru_desc), (None, None));
        assert_eq!(
            extract_salary_idr(None, "Gaji kompetitif untuk 13 juta pengguna dan berperan di 100 cabang"),
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
