use std::collections::HashSet;

use vecta_taxonomy::TaxonomyEngine;

struct BenchmarkCase {
    name: &'static str,
    title: &'static str,
    description: &'static str,
    expected_domain: &'static str,
    expected_skills: &'static [&'static str],
    forbidden_skills: &'static [&'static str],
}

const CASES: &[BenchmarkCase] = &[
    BenchmarkCase {
        name: "01_it_backend_laravel",
        title: "Backend Developer PHP Laravel",
        description: "Kualifikasi:\n- Menguasai PHP 8, Laravel, REST API, MySQL, dan Git.\n- Familiar dengan Docker sebagai nilai tambah.\nTentang Perusahaan:\nPerusahaan teknologi finansial di Bandung.",
        expected_domain: "informatika_data",
        expected_skills: &["PHP", "Laravel", "REST API", "MySQL", "Git", "Docker"],
        forbidden_skills: &["Java", "Go", "Audit Keuangan & Internal"],
    },
    BenchmarkCase {
        name: "02_finance_tax_brevet",
        title: "Tax & Accounting Staff",
        description: "Kualifikasi:\n- S1 Akuntansi, memiliki sertifikat Brevet A/B.\n- Berpengalaman menyusun Laporan Keuangan sesuai PSAK, rekonsiliasi bank, e-Faktur, e-Bupot, serta Accurate Online.\n- Menguasai Microsoft Excel (VLOOKUP, Pivot Table).",
        expected_domain: "ekonomi_akuntansi_pajak",
        expected_skills: &["Akuntansi", "Brevet Pajak A & B", "Laporan Keuangan", "PSAK / IFRS", "e-Faktur & e-Bupot", "Accurate Accounting", "Microsoft Excel"],
        forbidden_skills: &["Java", "Go", "Software QA Testing"],
    },
    BenchmarkCase {
        name: "03_marketing_digital",
        title: "Digital Marketing Specialist",
        description: "Persyaratan:\n- Berpengalaman mengelola Meta Ads, TikTok Ads, Google Ads, dan SEO.\n- Mampu membuat strategi Social Media Management dan Copywriting persuasif.",
        expected_domain: "bisnis_marketing_sales",
        expected_skills: &["Digital Marketing", "Meta Ads & TikTok Ads", "SEM & Google Ads", "SEO", "Social Media Management", "Copywriting"],
        forbidden_skills: &["Go", "Rust"],
    },
    BenchmarkCase {
        name: "04_hr_about_company_noise_guard",
        title: "HRGA & Payroll Officer",
        description: "Tentang Perusahaan:\nPT Awan Nusantara adalah perusahaan teknologi berbasis Kubernetes, Docker, Python, dan Go yang melayani jutaan pengguna.\n\nKualifikasi:\n- S1 Psikologi atau Manajemen SDM.\n- Menguasai proses End-to-End Recruitment, Payroll, BPJS Ketenagakerjaan, HRIS Talenta, dan administrasi psikotes.",
        expected_domain: "hr_psikologi_admin_ops",
        expected_skills: &["Rekrutmen & Seleksi", "Payroll & Kompensasi", "BPJS & Ketenagakerjaan", "HRIS", "Asesmen Psikologi & Psikotes"],
        forbidden_skills: &["Kubernetes", "Docker", "Python", "Go"],
    },
    BenchmarkCase {
        name: "05_civil_site_engineer",
        title: "Civil Site Engineer / Quantity Surveyor",
        description: "Requirements:\n- S1 Teknik Sipil.\n- Mahir AutoCAD, ETABS, Revit BIM, penyusunan RAB dan BoQ, serta Manajemen Konstruksi Kurva S.",
        expected_domain: "teknik_sipil_arsitektur_gis",
        expected_skills: &["AutoCAD", "ETABS", "Revit & BIM", "RAB & Quantity Surveying", "Manajemen Konstruksi"],
        forbidden_skills: &["SAP ERP", "Go"],
    },
    BenchmarkCase {
        name: "06_mechanical_k3_mfg",
        title: "Mechanical Maintenance & HSE Engineer",
        description: "Kualifikasi:\n- S1 Teknik Mesin / Elektro.\n- Berpengalaman SolidWorks, PLC Siemens, Preventive Maintenance, memiliki sertifikat Ahli K3 Umum Kemnaker, dan paham ISO 9001 serta Lean Manufacturing.",
        expected_domain: "teknik_mesin_elektro_industri",
        expected_skills: &["SolidWorks & Mechanical CAD", "PLC & Otomasi Industri", "Maintenance & Reliability", "K3 / HSE", "ISO 9001 / 14001 / 45001", "Lean Manufacturing & Six Sigma"],
        forbidden_skills: &["Software QA Testing"],
    },
    BenchmarkCase {
        name: "07_health_apoteker_cpob",
        title: "Apoteker Penanggung Jawab & Regulatory Affairs",
        description: "Kualifikasi:\n- Profesi Apoteker (STRA aktif).\n- Memahami CPOB, CPOTB, Registrasi BPOM, dan Farmakovigilans.",
        expected_domain: "kesehatan_farmasi_keperawatan",
        expected_skills: &["Farmasi Klinis & Apoteker", "CPOB / CPOTB / CDOB", "Registrasi BPOM & Regulatory Affairs"],
        forbidden_skills: &["Software QA Testing", "Go"],
    },
    BenchmarkCase {
        name: "08_legal_corporate",
        title: "Corporate Legal Officer",
        description: "Kualifikasi:\n- S1 Hukum.\n- Berpengalaman Legal Drafting, Contract Review, menyusun Legal Opinion, pengurusan perizinan OSS RBA/NIB, serta kepatuhan UU PDP.",
        expected_domain: "hukum_regulasi_kepatuhan",
        expected_skills: &["Legal Drafting & Review Kontrak", "Legal Opinion & Riset Hukum", "Perizinan Usaha & OSS/NIB", "Kepatuhan & Regulasi (Compliance)"],
        forbidden_skills: &["Go", "Rust"],
    },
    BenchmarkCase {
        name: "09_design_creative",
        title: "Graphic Designer & Video Editor",
        description: "Persyaratan:\n- Menguasai Adobe Photoshop, Adobe Illustrator, Premiere Pro, After Effects, dan Canva untuk kebutuhan kampanye visual.",
        expected_domain: "komunikasi_desain_media",
        expected_skills: &["Adobe Photoshop", "Adobe Illustrator", "Adobe Premiere Pro & Video Editing", "After Effects & Motion Graphics", "Canva & Desain Grafis"],
        forbidden_skills: &["Java", "SQL"],
    },
    BenchmarkCase {
        name: "10_food_tech_haccp",
        title: "QC Pangan & Food Safety Supervisor",
        description: "Kualifikasi:\n- S1 Teknologi Pangan.\n- Memahami penerapan HACCP, ISO 22000, Sertifikasi Halal SJPH, dan formulasi pangan.",
        expected_domain: "pertanian_pangan_lingkungan",
        expected_skills: &["HACCP & Keamanan Pangan", "Teknologi Pangan & R&D Produk"],
        forbidden_skills: &["Software QA Testing"],
    },
    BenchmarkCase {
        name: "11_education_kurikulum",
        title: "Guru Matematika & Instructional Designer",
        description: "Kualifikasi:\n- S1 Pendidikan.\n- Berpengalaman menyusun Modul Ajar Kurikulum Merdeka, manajemen kelas, asesmen pembelajaran, dan LMS Moodle.",
        expected_domain: "pendidikan_pelatihan",
        expected_skills: &["Kurikulum Merdeka & Perencanaan Ajar", "Pengajaran & Manajemen Kelas", "Instructional Design & E-Learning"],
        forbidden_skills: &["Go"],
    },
    BenchmarkCase {
        name: "12_hospitality_hotel",
        title: "Front Office Supervisor",
        description: "Requirements:\n- Diploma/S1 Pariwisata atau Perhotelan.\n- Menguasai sistem Opera PMS / VHP, Guest Relations, serta Bahasa Inggris aktif (TOEFL/IELTS).",
        expected_domain: "pariwisata_perhotelan_fnb",
        expected_skills: &["Front Office & Hotel PMS", "Bahasa Inggris"],
        forbidden_skills: &["PHP", "Go"],
    },
    BenchmarkCase {
        name: "13_science_lab_hplc",
        title: "Analis Laboratorium Kimia",
        description: "Kualifikasi:\n- S1 Kimia / Analis Kimia.\n- Mengoperasikan HPLC, GC-MS, Spektrofotometri UV-Vis, serta memahami standar pengujian laboratorium ISO 17025 dan pengolahan data SPSS.",
        expected_domain: "sains_lab_geologi_energi",
        expected_skills: &["HPLC / GC-MS & Analisis Instrumen", "Pengujian Laboratorium (ISO 17025)", "Statistika & SPSS / R"],
        forbidden_skills: &["Java", "Go"],
    },
    BenchmarkCase {
        name: "14_supply_chain_exim",
        title: "Export Import & Logistics Specialist",
        description: "Kualifikasi:\n- Pengalaman mengurus PIB, PEB, HS Code, Incoterms, Freight Forwarding, Warehouse Management, dan Negosiasi vendor.",
        expected_domain: "hr_psikologi_admin_ops",
        expected_skills: &["Ekspor-Impor & Kepabeanan", "Logistik & Warehouse", "Negosiasi & Kepemimpinan"],
        forbidden_skills: &["PHP", "Go"],
    },
];

#[test]
fn cross_domain_indonesian_benchmark_meets_precision_and_recall_gates() {
    let engine = TaxonomyEngine::global();
    assert!(
        engine.all_skills().len() >= 140,
        "taxonomy must cover rich cross-domain Indonesian skills"
    );

    let mut tp = 0usize;
    let mut fp = 0usize;
    let mut fn_count = 0usize;

    for case in CASES {
        let out = engine.extract_job(case.title, case.description, None);
        assert_eq!(
            out.dominant_domain, case.expected_domain,
            "case '{}' domain mismatch: got '{}'",
            case.name, out.dominant_domain
        );

        let found: HashSet<&str> = out.skills.iter().map(|s| s.canonical_name.as_str()).collect();

        for &exp in case.expected_skills {
            if found.contains(exp) {
                tp += 1;
            } else {
                fn_count += 1;
                eprintln!("[BENCHMARK MISS] case={} missing='{}' found={:?}", case.name, exp, found);
            }
        }
        for &forb in case.forbidden_skills {
            if found.contains(forb) {
                fp += 1;
                eprintln!("[BENCHMARK FP] case={} forbidden='{}' wrongly extracted!", case.name, forb);
            }
        }
        // Any extra unexpected skill outside expected_skills is counted conservatively only if forbidden,
        // while we also verify zero forbidden false positives across all 14 domains.
        assert!(
            case.forbidden_skills.iter().all(|f| !found.contains(*f)),
            "case '{}' leaked forbidden skill: {:?}",
            case.name,
            found
        );
    }

    let precision = tp as f64 / (tp + fp).max(1) as f64;
    let recall = tp as f64 / (tp + fn_count).max(1) as f64;
    assert!(
        precision >= 0.95,
        "Taxonomy precision {precision:.3} below 0.95 gate (tp={tp}, fp={fp})"
    );
    assert!(
        recall >= 0.92,
        "Taxonomy recall {recall:.3} below 0.92 gate (tp={tp}, fn={fn_count})"
    );
}

#[test]
fn parent_child_hierarchy_resolves_across_domains() {
    let engine = TaxonomyEngine::global();
    assert_eq!(engine.parent_canonical_name("Laravel"), Some("PHP"));
    assert_eq!(
        engine.parent_canonical_name("Brevet Pajak A & B"),
        Some("Perpajakan")
    );
    assert_eq!(
        engine.parent_canonical_name("ETABS"),
        Some("Analisis Struktur Sipil")
    );
    assert_eq!(
        engine.parent_canonical_name("SEO"),
        Some("Digital Marketing")
    );
}
