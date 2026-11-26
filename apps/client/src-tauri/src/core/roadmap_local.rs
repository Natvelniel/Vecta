use std::collections::HashMap;

use vecta_taxonomy::TaxonomyEngine;

use crate::models::roadmap::{GeneratedRoadmap, GeneratedRoadmapStep};

fn normalize_domain_label(domain: &str) -> &'static str {
    match domain {
        "informatika_data" => "Teknologi & Data",
        "ekonomi_akuntansi_pajak" | "perbankan_aktuaria_asuransi" => "Keuangan & Perbankan",
        "bisnis_marketing_sales" => "Bisnis & Pemasaran",
        "hr_psikologi_admin_ops" | "administrasi_publik_hi_sosial" => "Operasional & SDM",
        "teknik_sipil_arsitektur_gis" => "Konstruksi & Infrastruktur",
        "teknik_mesin_elektro_industri"
        | "teknik_elektro_otomasi_telko"
        | "teknik_industri_k3_mutu" => "Rekayasa & Manufaktur",
        "kesehatan_farmasi_keperawatan" | "kedokteran_keperawatan_gizi_kesmas" => {
            "Kesehatan & Klinis"
        }
        "hukum_regulasi_kepatuhan" => "Hukum & Kepatuhan",
        "komunikasi_desain_media" | "dkv_produk_ux_kreatif" | "bahasa_sastra_penerjemahan" => {
            "Kreatif, Media & Bahasa"
        }
        "pertanian_pangan_lingkungan" | "sains_lab_geologi_energi" | "pertambangan_migas_energi" => {
            "Sains, Pangan & Energi"
        }
        "pendidikan_pelatihan" => "Pendidikan & Pelatihan",
        "pariwisata_perhotelan_fnb" => "Hospitality & Layanan",
        "supply_chain_logistik_exim" => "Rantai Pasok & Logistik",
        _ => "Lintas Bidang",
    }
}

fn build_mini_project(domain: &str, skill: &str, role: &str, phase_order: i32) -> String {
    match domain {
        "ekonomi_akuntansi_pajak" | "perbankan_aktuaria_asuransi" => match phase_order {
            1 => format!(
                "Susun kertas kerja transaksi dan rekonsiliasi dasar menggunakan {skill} untuk studi kasus UKM sesuai posisi {role}."
            ),
            2 => format!(
                "Simulasikan pelaporan periodik dan audit trail memakai {skill} dengan template neraca serta arus kas."
            ),
            _ => format!(
                "Siapkan portofolio studi kasus analisis kepatuhan & efisiensi biaya berbasis {skill} dalam format PDF ringkas."
            ),
        },
        "bisnis_marketing_sales" => match phase_order {
            1 => format!(
                "Petakan funnel audiens dan metrik utama {skill} untuk satu produk lokal pada peran {role}."
            ),
            2 => format!(
                "Buat draf kampanye lengkap beserta A/B copy dan kalender eksekusi menggunakan {skill}."
            ),
            _ => format!(
                "Susun studi kasus evaluasi ROI/konversi berbasis {skill} lengkap dengan rekomendasi optimasi."
            ),
        },
        "teknik_sipil_arsitektur_gis"
        | "teknik_mesin_elektro_industri"
        | "teknik_elektro_otomasi_telko"
        | "teknik_industri_k3_mutu" => match phase_order {
            1 => format!(
                "Pahami standar teknis, notasi gambar, dan parameter kerja {skill} untuk lingkup {role}."
            ),
            2 => format!(
                "Kerjakan satu gambar kerja / perhitungan beban / BoQ mini menerapkan {skill} dengan verifikasi mandiri."
            ),
            _ => format!(
                "Dokumentasikan portofolio proyek rekayasa berbasis {skill} lengkap dengan checklist mutu/K3."
            ),
        },
        "kesehatan_farmasi_keperawatan" | "kedokteran_keperawatan_gizi_kesmas" => match phase_order {
            1 => format!(
                "Ringkas protokol klinis dan standar keselamatan pasien terkait {skill} untuk peran {role}."
            ),
            2 => format!(
                "Latih pencatatan kasus SOAP / verifikasi formularium secara terstruktur memakai {skill}."
            ),
            _ => format!(
                "Susun refleksi studi kasus klinis terukur untuk menunjukkan kesiapan praktik {skill} saat wawancara."
            ),
        },
        "hukum_regulasi_kepatuhan" => match phase_order {
            1 => format!(
                "Inventarisasi pasal regulasi terbaru dan istilah kunci yang relevan dengan {skill}."
            ),
            2 => format!(
                "Susun satu draf perjanjian kerja sama atau legal memorandum singkat mengacu pada {skill}."
            ),
            _ => format!(
                "Rangkum matriks mitigasi risiko kepatuhan berbasis {skill} sebagai portofolio {role}."
            ),
        },
        "komunikasi_desain_media" | "dkv_produk_ux_kreatif" => match phase_order {
            1 => format!(
                "Analisis 3 studi kasus terbaik di industri yang memanfaatkan {skill} secara efektif."
            ),
            2 => format!(
                "Produksi 1 aset karya / prototipe end-to-end dengan {skill} yang selaras kebutuhan {role}."
            ),
            _ => format!(
                "Publikasikan breakdown proses berpikir (masalah -> solusi -> hasil) untuk karya {skill} di portofolio."
            ),
        },
        "informatika_data" => match phase_order {
            1 => format!(
                "Bangun eksperimen terisolasi dan catatan arsitektur inti menggunakan {skill} untuk skenario {role}."
            ),
            2 => format!(
                "Integrasikan {skill} ke dalam proyek mini dengan pengujian otomatis, error handling, dan README jelas."
            ),
            _ => format!(
                "Lakukan benchmarking/audit keandalan serta rangkum metrik capaian {skill} di portofolio GitHub/CV."
            ),
        },
        _ => match phase_order {
            1 => format!(
                "Pahami konsep inti, SOP kerja, dan terminologi standar {skill} untuk posisi {role}."
            ),
            2 => format!(
                "Terapkan {skill} pada studi kasus simulasi pekerjaan nyata beserta daftar periksa kualitas."
            ),
            _ => format!(
                "Rangkum hasil studi kasus {skill} menjadi 2 poin pencapaian terukur untuk CV dan wawancara."
            ),
        },
    }
}

pub fn generate_local_roadmap(
    target_role: &str,
    missing_skills: &[String],
    hours_per_week: u32,
    taxonomy: &TaxonomyEngine,
) -> GeneratedRoadmap {
    let role = if target_role.trim().is_empty() {
        "Profesional Target"
    } else {
        target_role.trim()
    };

    let skill_domain_map: HashMap<String, &str> = taxonomy
        .all_skills()
        .iter()
        .map(|s| (s.canonical_name.to_lowercase(), s.domain.as_str()))
        .collect();

    let selected_skills: Vec<String> = missing_skills
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .take(6)
        .collect();

    let total_skills = selected_skills.len().max(1);
    let mut steps = Vec::with_capacity(selected_skills.len());
    let mut total_hours = 0i32;

    for (idx, skill) in selected_skills.iter().enumerate() {
        let phase_order = if total_skills <= 2 {
            (idx as i32) + 1
        } else {
            let third = (total_skills + 2) / 3;
            if idx < third {
                1
            } else if idx < third * 2 {
                2
            } else {
                3
            }
        };

        let phase_name = match phase_order {
            1 => "Fase 1 · Fondasi Inti",
            2 => "Fase 2 · Praktik Terapan",
            _ => "Fase 3 · Portofolio & Simulasi",
        };

        let domain = skill_domain_map
            .get(&skill.to_lowercase())
            .copied()
            .unwrap_or("umum_lintas_bidang");
        let domain_label = normalize_domain_label(domain);

        let estimated_hours = match phase_order {
            1 => 6,
            2 => 8,
            _ => 10,
        };
        total_hours += estimated_hours;

        let topic_title = match phase_order {
            1 => format!("Kuasai Konsep Dasar & Alur Kerja {skill}"),
            2 => format!("Terapkan {skill} dalam Studi Kasus {role}"),
            _ => format!("Bukti Portofolio & Simulasi Kerja {skill}"),
        };

        let learning_keywords = vec![
            format!("tutorial {skill} bahasa indonesia"),
            format!("studi kasus {skill} {role}"),
            format!("checklist praktik {skill} {domain_label}"),
        ];

        steps.push(GeneratedRoadmapStep {
            phase_order,
            phase_name: phase_name.to_string(),
            target_skill: skill.clone(),
            topic_title,
            learning_keywords,
            mini_project_idea: build_mini_project(domain, skill, role, phase_order),
            estimated_hours,
        });
    }

    let weekly = (hours_per_week.max(2)) as i32;
    let total_estimated_weeks = ((total_hours + weekly - 1) / weekly).max(1);

    GeneratedRoadmap {
        title: format!("Roadmap {role} (Lokal · 0 Token)"),
        target_role: role.to_string(),
        total_estimated_weeks,
        steps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_local_roadmap_across_non_it_and_it_domains() {
        let taxonomy = TaxonomyEngine::global();

        let non_it = generate_local_roadmap(
            "Akuntan Pajak",
            &[
                "PSAK".into(),
                "e-Faktur".into(),
                "PPh 21".into(),
                "Rekonsiliasi Fiskal".into(),
            ],
            8,
            taxonomy,
        );
        assert!(non_it.title.contains("Lokal · 0 Token"));
        assert!(!non_it.title.contains('\u{2014}'));
        assert_eq!(non_it.steps.len(), 4);
        assert!(non_it.total_estimated_weeks >= 1);
        assert_eq!(non_it.steps[0].target_skill, "PSAK");
        assert!(!non_it.steps[0].mini_project_idea.is_empty());

        let it = generate_local_roadmap(
            "Backend Engineer",
            &["Docker".into(), "Kubernetes".into()],
            10,
            taxonomy,
        );
        assert_eq!(it.steps.len(), 2);
        assert_eq!(it.steps[0].target_skill, "Docker");
        assert_eq!(it.steps[1].phase_order, 2);
    }
}
