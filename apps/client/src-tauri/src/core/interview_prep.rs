use crate::models::job::{InterviewPrepKit, InterviewQuestionItem};

pub fn build_local_interview_prep(
    job_id: &str,
    job_title: &str,
    company: &str,
    matched_skills: &[String],
    missing_skills: &[String],
    language: &str,
) -> InterviewPrepKit {
    let is_en = language.trim().eq_ignore_ascii_case("en");
    let primary_skill = matched_skills
        .first()
        .map(String::as_str)
        .unwrap_or("keahlian inti");
    let secondary_skill = matched_skills
        .get(1)
        .map(String::as_str)
        .unwrap_or(primary_skill);
    let top_gap = missing_skills
        .first()
        .map(String::as_str)
        .unwrap_or("alur kerja internal");

    if is_en {
        let pitch_30s = format!(
            "I am a candidate targeting {job_title} at {company}, bringing hands-on experience in {primary_skill} and {secondary_skill} with a focus on measurable, reliable outcomes."
        );
        let questions = vec![
            InterviewQuestionItem {
                category: "Behavioral STAR".into(),
                question: format!(
                    "Tell me about a high-stakes problem you solved using {primary_skill} and how you measured the impact."
                ),
                star_hint: format!(
                    "Situation: Brief project context. Task: Core bottleneck. Action: Exact steps you took with {primary_skill}. Result: Concrete metric or time saved."
                ),
                focus_skill: Some(primary_skill.to_string()),
            },
            InterviewQuestionItem {
                category: "Technical / Domain".into(),
                question: format!(
                    "Walk me through your workflow when applying {secondary_skill} in a production or team setting for {job_title}."
                ),
                star_hint: "Highlight quality checks, collaboration, edge cases you anticipated, and how you documented your work.".into(),
                focus_skill: Some(secondary_skill.to_string()),
            },
            InterviewQuestionItem {
                category: "Collaboration STAR".into(),
                question: "Describe a time requirements changed mid-project or you disagreed with a teammate on an approach.".into(),
                star_hint: "Focus on objective evidence, listening calmly, aligning on team goals, and what shipped on time.".into(),
                focus_skill: None,
            },
            InterviewQuestionItem {
                category: "Gap Mitigation".into(),
                question: if missing_skills.is_empty() {
                    format!("How do you ramp up quickly on unfamiliar internal tools when joining {company}?")
                } else {
                    format!("This role lists {top_gap}, which is newer on your profile. How would you ramp up in your first 30 days?")
                },
                star_hint: if missing_skills.is_empty() {
                    "Show a repeatable 2-week learning framework: docs -> small sandbox task -> peer review.".into()
                } else {
                    format!("Connect adjacent strength in {primary_skill} to {top_gap}, and outline a 30-day ramp-up plan with a concrete deliverable.")
                },
                focus_skill: missing_skills.first().cloned(),
            },
            InterviewQuestionItem {
                category: "Execution & Ownership".into(),
                question: format!(
                    "If selected as {job_title}, what would be your priority during week 1 and month 1 at {company}?"
                ),
                star_hint: "Week 1: Learn context & stakeholders. Week 2-3: Deliver a low-risk win. Month 1: Establish measurable ownership.".into(),
                focus_skill: None,
            },
            InterviewQuestionItem {
                category: "Role Motivation".into(),
                question: format!("Why {job_title} at {company} specifically right now?"),
                star_hint: "Tie the company's domain/scale to 1-2 strengths in your background and how you can contribute immediately.".into(),
                focus_skill: None,
            },
        ];
        let closing_questions = vec![
            format!("What does success look like in the first 90 days for this {job_title} role?"),
            "How does the team typically review work, share feedback, and collaborate across functions?".into(),
            "What is the biggest bottleneck the team hopes this hire will help unlock?".into(),
        ];
        return InterviewPrepKit {
            job_id: job_id.to_string(),
            language: "en".into(),
            pitch_30s,
            questions,
            closing_questions,
        };
    }

    let pitch_30s = format!(
        "Saya kandidat untuk posisi {job_title} di {company} dengan pengalaman langsung pada {primary_skill} dan {secondary_skill}, terbiasa bekerja rapi, terukur, dan kolaboratif lintas tim."
    );
    let questions = vec![
        InterviewQuestionItem {
            category: "Behavioral STAR".into(),
            question: format!(
                "Ceritakan proyek atau masalah tersulit yang Anda selesaikan menggunakan {primary_skill}, dan bagaimana Anda mengukur hasilnya."
            ),
            star_hint: format!(
                "Situation: Konteks singkat. Task: Target Anda. Action: Langkah konkret memakai {primary_skill}. Result: Dampak terukur (waktu/biaya/kualitas)."
            ),
            focus_skill: Some(primary_skill.to_string()),
        },
        InterviewQuestionItem {
            category: "Teknis & Alur Kerja".into(),
            question: format!(
                "Bagaimana alur kerja Anda sehari-hari saat menerapkan {secondary_skill} untuk peran {job_title}?"
            ),
            star_hint: "Jelaskan standar pengecekan kualitas, cara menghindari kesalahan umum, serta koordinasi dengan tim lain.".into(),
            focus_skill: Some(secondary_skill.to_string()),
        },
        InterviewQuestionItem {
            category: "Kolaborasi STAR".into(),
            question: "Pernahkah target pekerjaan berubah di tengah jalan atau ada perbedaan pendapat di tim? Bagaimana Anda menyikapinya?".into(),
            star_hint: "Tekankan komunikasi berbasis data, mendengarkan alasan pihak lain, lalu mencari titik temu agar target tetap tercapai.".into(),
            focus_skill: None,
        },
        InterviewQuestionItem {
            category: "Mitigasi Celah Skill".into(),
            question: if missing_skills.is_empty() {
                format!("Bagaimana strategi Anda beradaptasi cepat dengan SOP atau alat internal baru di {company}?")
            } else {
                format!("Posisi ini membutuhkan {top_gap} yang belum dominan di profil Anda. Bagaimana rencana Anda menguasainya di 30 hari pertama?")
            },
            star_hint: if missing_skills.is_empty() {
                "Tunjukkan pola belajar terstruktur: pelajari dokumentasi inti -> coba kasus kecil -> minta umpan balik.".into()
            } else {
                format!("Kaitkan pengalaman Anda di {primary_skill} sebagai fondasi ke {top_gap}, lalu sebutkan target nyata dalam 2-4 minggu awal.")
            },
            focus_skill: missing_skills.first().cloned(),
        },
        InterviewQuestionItem {
            category: "Prioritas 30 Hari".into(),
            question: format!(
                "Jika bergabung sebagai {job_title} di {company}, apa fokus utama Anda pada minggu pertama hingga bulan pertama?"
            ),
            star_hint: "Minggu 1: Pahami ekspektasi & SOP. Minggu 2-3: Selesaikan tugas berdampak cepat. Bulan 1: Beri laporan progres terukur.".into(),
            focus_skill: None,
        },
        InterviewQuestionItem {
            category: "Motivasi Peran".into(),
            question: format!("Mengapa Anda tertarik melamar sebagai {job_title} di {company}?"),
            star_hint: "Hubungkan bidang usaha perusahaan dengan 2 kekuatan utama Anda serta kesiapan memberi kontribusi jangka menengah.".into(),
            focus_skill: None,
        },
    ];
    let closing_questions = vec![
        format!("Seperti apa tolok ukur keberhasilan posisi {job_title} dalam 90 hari pertama?"),
        "Bagaimana ritme kolaborasi dan umpan balik di tim yang akan saya masuki?".into(),
        "Tantangan prioritas apa yang paling ingin diselesaikan tim melalui posisi ini?".into(),
    ];

    InterviewPrepKit {
        job_id: job_id.to_string(),
        language: "id".into(),
        pitch_30s,
        questions,
        closing_questions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_id_and_en_star_prep_with_6_questions() {
        let id_kit = build_local_interview_prep(
            "j-1",
            "Akuntan Pajak",
            "PT Maju",
            &["PSAK".into(), "PPh 21".into()],
            &["e-Faktur".into()],
            "id",
        );
        assert_eq!(id_kit.language, "id");
        assert_eq!(id_kit.questions.len(), 6);
        assert!(id_kit.pitch_30s.contains("PSAK"));
        assert!(!id_kit.pitch_30s.contains('\u{2014}'));

        let en_kit = build_local_interview_prep(
            "j-2",
            "Backend Engineer",
            "PT Karya",
            &["Rust".into(), "SQLite".into()],
            &[],
            "en",
        );
        assert_eq!(en_kit.language, "en");
        assert_eq!(en_kit.questions.len(), 6);
        assert!(en_kit.questions.iter().all(|q| !q.star_hint.is_empty()));
    }
}
