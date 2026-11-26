/// Normalize job titles across all 14 Indonesian academic and professional domains so filters
/// and Competency Maps stay clean for IT and non-IT careers alike.
pub fn normalize_role_universal(
    title: &str,
    dominant_domain: &str,
    query_hint: Option<&str>,
) -> String {
    let t = title.to_lowercase();

    // 1. Informatika & Data
    if t.contains("full stack") || t.contains("fullstack") || t.contains("full-stack") {
        return "Full Stack Developer".into();
    }
    if t.contains("backend") || t.contains("back end") || t.contains("back-end") {
        return "Backend Developer".into();
    }
    if t.contains("frontend") || t.contains("front end") || t.contains("front-end") {
        return "Frontend Developer".into();
    }
    if t.contains("data scientist") || t.contains("machine learning") || t.contains("ai engineer") {
        return "Data & AI Engineer".into();
    }
    if t.contains("data analyst") || t.contains("business intelligence") {
        return "Data Analyst".into();
    }
    if t.contains("data engineer") || t.contains("database") || t.contains("dba") {
        return "Database & Data Engineer".into();
    }
    if t.contains("devops") || t.contains("site reliability") || t.contains("sre") || t.contains("cloud engineer") {
        return "DevOps & Cloud Engineer".into();
    }
    if t.contains("mobile developer") || t.contains("android developer") || t.contains("ios developer") || t.contains("flutter") {
        return "Mobile Developer".into();
    }
    if t.contains("cyber") || t.contains("information security") || t.contains("security engineer") {
        return "Cybersecurity Engineer".into();
    }
    if t.contains("it support") || t.contains("network engineer") || t.contains("system administrator") {
        return "IT Support & Network".into();
    }
    if t.contains("ui/ux") || t.contains("product designer") || t.contains("ux designer") || t.contains("ui designer") {
        return "UI/UX Designer".into();
    }
    if t.contains("qa engineer") || t.contains("software tester") || t.contains("qa automation") {
        return "Software QA Engineer".into();
    }
    if t.contains("web developer") || t.contains("web programmer") || t.contains("laravel") || t.contains("php developer") {
        return "Backend Developer".into();
    }
    if t.contains("software engineer") || t.contains("software developer") || t.contains("programmer") {
        return "Software Engineer".into();
    }

    // 2. Ekonomi, Akuntansi, Pajak & Keuangan
    if t.contains("tax") || t.contains("pajak") || t.contains("perpajakan") {
        return "Tax & Perpajakan".into();
    }
    if t.contains("auditor") || t.contains("internal audit") || t.contains("audit") {
        return "Audit & Pengendalian Internal".into();
    }
    if t.contains("accountant") || t.contains("akuntan") || t.contains("accounting") || t.contains("akuntansi") || t.contains("finance") || t.contains("keuangan") {
        return "Akuntansi & Keuangan".into();
    }
    if t.contains("credit analyst") || t.contains("analis kredit") || t.contains("risk") {
        return "Analisis Kredit & Risiko".into();
    }

    // 3. Bisnis, Marketing & Sales
    if t.contains("digital marketing") || t.contains("performance marketing") || t.contains("seo") || t.contains("social media") {
        return "Digital Marketing".into();
    }
    if t.contains("copywriter") || t.contains("content writer") || t.contains("content creator") {
        return "Content & Copywriting".into();
    }
    if t.contains("business development") || t.contains("partnership") {
        return "Business Development".into();
    }
    if t.contains("account executive") || t.contains("sales") || t.contains("penjualan") || t.contains("marketing") {
        return "Sales & Marketing".into();
    }

    // 4. HR, Psikologi, Administrasi & Operasional
    if t.contains("recruiter") || t.contains("talent acquisition") || t.contains("hrd") || t.contains("human resource") || t.contains("hrga") || t.contains("payroll") || t.contains("people") {
        return "HR & Rekrutmen".into();
    }
    if t.contains("supply chain") || t.contains("ppic") || t.contains("procurement") || t.contains("purchasing") || t.contains("warehouse") || t.contains("logistik") || t.contains("logistics") || t.contains("exim") {
        return "Supply Chain & Logistik".into();
    }
    if t.contains("customer service") || t.contains("customer support") || t.contains("call center") {
        return "Customer Service".into();
    }
    if t.contains("admin") || t.contains("administrasi") || t.contains("sekretaris") || t.contains("document control") {
        return "Administrasi & Operasional".into();
    }

    // 5. Teknik Sipil, Arsitektur & GIS
    if t.contains("arsitek") || t.contains("architect") || t.contains("interior") {
        return "Arsitektur & Interior".into();
    }
    if t.contains("quantity surveyor") || t.contains("site engineer") || t.contains("civil") || t.contains("sipil") || t.contains("konstruksi") || t.contains("drafter") || t.contains("gis") || t.contains("surveyor") {
        return "Teknik Sipil & Konstruksi".into();
    }

    // 6. Teknik Mesin, Elektro, Industri & K3
    if t.contains("hse") || t.contains("k3") || t.contains("safety officer") {
        return "K3 & HSE".into();
    }
    if t.contains("mechanical") || t.contains("teknik mesin") || t.contains("electrical") || t.contains("elektro") || t.contains("maintenance") || t.contains("teknisi") || t.contains("automation") || t.contains("plc") {
        return "Teknik Mesin, Elektro & Maintenance".into();
    }
    if t.contains("quality control") || t.contains("qc ") || t.ends_with(" qc") || t.contains("quality assurance") || t.contains("produksi") || t.contains("industrial engineer") {
        if dominant_domain == "pertanian_pangan_lingkungan" {
            return "Quality Control & Pangan".into();
        }
        if dominant_domain == "kesehatan_farmasi_keperawatan" {
            return "Farmasi & QA Kesehatan".into();
        }
        return "Quality Control & Manufaktur".into();
    }

    // 7. Kesehatan, Farmasi & Keperawatan
    if t.contains("apoteker") || t.contains("farmasi") || t.contains("pharmacist") || t.contains("regulatory affairs") {
        return "Farmasi & Regulatory".into();
    }
    if t.contains("perawat") || t.contains("nurse") || t.contains("bidan") || t.contains("dokter") || t.contains("rekam medis") || t.contains("ahli gizi") {
        return "Kesehatan & Medis".into();
    }

    // 8. Hukum & Kepatuhan
    if t.contains("legal") || t.contains("hukum") || t.contains("compliance") || t.contains("advokat") || t.contains("notaris") {
        return "Hukum & Kepatuhan".into();
    }

    // 9. Komunikasi, Desain & Media
    if t.contains("graphic design") || t.contains("desain grafis") || t.contains("video editor") || t.contains("motion") || t.contains("fotografer") {
        return "Desain Grafis & Multimedia".into();
    }
    if t.contains("public relations") || t.contains("corporate comm") || t.contains("jurnalis") || t.contains("reporter") {
        return "Komunikasi & Media".into();
    }

    // 10. Pertanian, Pangan & Lingkungan
    if t.contains("agronom") || t.contains("perkebunan") || t.contains("pertanian") || t.contains("pangan") || t.contains("lingkungan") || t.contains("environmental") {
        return "Pertanian, Pangan & Lingkungan".into();
    }

    // 11. Pendidikan & Pelatihan
    if t.contains("guru") || t.contains("teacher") || t.contains("dosen") || t.contains("tutor") || t.contains("trainer") || t.contains("instruktur") {
        return "Pendidikan & Pelatihan".into();
    }

    // 12. Pariwisata, Perhotelan & F&B
    if t.contains("front office") || t.contains("housekeeping") || t.contains("hotel") || t.contains("barista") || t.contains("chef") || t.contains("f&b") || t.contains("event") {
        return "Perhotelan, F&B & Event".into();
    }

    // 13. Sains, Lab, Geologi, Tambang & Migas
    if t.contains("tambang") || t.contains("mining") || t.contains("geolog") || t.contains("migas") || t.contains("drilling") || t.contains("reservoir") || t.contains("petrel") {
        return "Pertambangan, Geologi & Migas".into();
    }
    if t.contains("laboratorium") || t.contains("lab analyst") || t.contains("analis kimia") {
        return "Sains & Laboratorium".into();
    }

    // 14. Perbankan, Aktuaria & Asuransi
    if t.contains("aktuaria") || t.contains("actuary") || t.contains("underwriter") || t.contains("relationship manager") || t.contains("treasury") || t.contains("kredit") || t.contains("teller") {
        return "Perbankan, Aktuaria & Asuransi".into();
    }

    // 15. Product Management & Strategi
    if t.contains("product manager") || t.contains("product owner") || t.contains("scrum master") || t.contains("bizops") {
        return "Product Management & Strategi".into();
    }

    // 16. Kebijakan Publik, HI & NGO
    if t.contains("policy") || t.contains("kebijakan") || t.contains("hubungan internasional") || t.contains("ngo") || t.contains("csr") || t.contains("community dev") {
        return "Sektor Publik, Kebijakan & NGO".into();
    }

    // 17. Bahasa, Sastra & Penerjemahan
    if t.contains("translator") || t.contains("penerjemah") || t.contains("interpreter") || t.contains("localization") || t.contains("technical writer") {
        return "Bahasa & Penerjemahan".into();
    }

    // 18. Telekomunikasi, RF & Fiber
    if t.contains("rf ") || t.contains("telco") || t.contains("telekomunikasi") || t.contains("ftth") || t.contains("fiber optic") {
        return "Telekomunikasi & Jaringan".into();
    }

    query_hint
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .map(|q| q.chars().take(60).collect())
        .unwrap_or_else(|| title.trim().chars().take(60).collect())
}
