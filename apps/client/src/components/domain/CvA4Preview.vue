<script setup lang="ts">
import { computed } from "vue";
import type { MasterCvProfile } from "@/lib/bindings";

const props = defineProps<{
  cv: MasterCvProfile;
  skills: string[];
  highlightedKeywords?: string[];
  lang?: "id" | "en";
}>();

const labels = computed(() =>
  props.lang === "en"
    ? {
        summary: "Professional Summary",
        skills: "Core Skills",
        experience: "Experience",
        projects: "Projects & Portfolio",
        education: "Education",
        additional: "Certifications & Languages",
      }
    : {
        summary: "Ringkasan Profesional",
        skills: "Keahlian Utama",
        experience: "Pengalaman Kerja",
        projects: "Proyek & Portofolio",
        education: "Pendidikan",
        additional: "Sertifikasi & Bahasa",
      },
);

const highlightSet = computed(
  () => new Set((props.highlightedKeywords ?? []).map((s) => s.toLowerCase())),
);

const contactLine = computed(() =>
  [props.cv.location, props.cv.phone, props.cv.email, props.cv.linkedin_url]
    .map((v) => v.trim())
    .filter(Boolean)
    .join("  |  "),
);
</script>

<template>
  <div class="v-a4-wrap" aria-label="Pratinjau langsung kertas A4 CV ATS">
    <article class="v-a4-sheet">
      <header class="v-a4-head">
        <h2 class="v-a4-name">{{ cv.full_name.trim() || "Nama Lengkap Kandidat" }}</h2>
        <p v-if="cv.headline.trim()" class="v-a4-headline">{{ cv.headline }}</p>
        <p v-if="contactLine" class="v-a4-contact">{{ contactLine }}</p>
      </header>

      <section v-if="cv.summary.trim()" class="v-a4-sec">
        <h3 class="v-a4-h3">{{ labels.summary }}</h3>
        <p class="v-a4-p">{{ cv.summary }}</p>
      </section>

      <section v-if="skills.length > 0" class="v-a4-sec">
        <h3 class="v-a4-h3">{{ labels.skills }}</h3>
        <p class="v-a4-p">
          <span v-for="(sk, i) in skills" :key="sk">
            <strong
              :class="{ 'v-a4-kw-hit': highlightSet.has(sk.toLowerCase()) }"
            >{{ sk }}</strong><span v-if="i < skills.length - 1">, </span>
          </span>
        </p>
      </section>

      <section v-if="cv.experiences.length > 0" class="v-a4-sec">
        <h3 class="v-a4-h3">{{ labels.experience }}</h3>
        <div v-for="(exp, i) in cv.experiences" :key="i" class="v-a4-item">
          <div class="v-a4-item-top">
            <strong>{{ exp.role || "Posisi" }}</strong>
            <span>{{ exp.period }}</span>
          </div>
          <div class="v-a4-sub">{{ [exp.company, exp.location].filter(Boolean).join(" · ") }}</div>
          <ul v-if="exp.bullets.filter(Boolean).length > 0" class="v-a4-ul">
            <li v-for="(b, bi) in exp.bullets.filter(Boolean)" :key="bi">{{ b }}</li>
          </ul>
        </div>
      </section>

      <section v-if="cv.projects.length > 0" class="v-a4-sec">
        <h3 class="v-a4-h3">{{ labels.projects }}</h3>
        <div v-for="(proj, i) in cv.projects" :key="i" class="v-a4-item">
          <div class="v-a4-item-top">
            <strong>{{ proj.name || "Proyek" }}</strong>
            <span>{{ proj.period }}</span>
          </div>
          <p v-if="proj.description" class="v-a4-p">{{ proj.description }}</p>
        </div>
      </section>

      <section v-if="cv.education.length > 0" class="v-a4-sec">
        <h3 class="v-a4-h3">{{ labels.education }}</h3>
        <div v-for="(edu, i) in cv.education" :key="i" class="v-a4-item">
          <div class="v-a4-item-top">
            <strong>{{ edu.institution || "Institusi" }}</strong>
            <span>{{ edu.period }}</span>
          </div>
          <div class="v-a4-sub">{{ [edu.degree, edu.notes].filter(Boolean).join(" · ") }}</div>
        </div>
      </section>

      <section
        v-if="cv.certifications.length > 0 || cv.languages.length > 0"
        class="v-a4-sec"
      >
        <h3 class="v-a4-h3">{{ labels.additional }}</h3>
        <p v-if="cv.certifications.length > 0" class="v-a4-p">
          <strong>Sertifikasi:</strong> {{ cv.certifications.join(", ") }}
        </p>
        <p v-if="cv.languages.length > 0" class="v-a4-p">
          <strong>Bahasa:</strong> {{ cv.languages.join(", ") }}
        </p>
      </section>
    </article>
  </div>
</template>

<style scoped>
.v-a4-wrap {
  width: 100%;
  display: flex;
  justify-content: center;
}
/* Paper sheet stays faithful printed A4 white in both Dark & Light app themes */
.v-a4-sheet {
  width: 100%;
  max-width: 560px;
  min-height: 680px;
  background: #ffffff;
  color: #111827;
  border-radius: 6px;
  border: 1px solid var(--border);
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.24);
  padding: 28px 30px;
  font-family: var(--font-primary);
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.v-a4-head {
  border-bottom: 1.5px solid #111827;
  padding-bottom: 8px;
}
.v-a4-name {
  margin: 0;
  font-size: 18px;
  font-weight: 700;
  letter-spacing: -0.02em;
  color: #09090b;
}
.v-a4-headline {
  margin: 2px 0 0;
  font-size: 12px;
  font-weight: 600;
  color: #374151;
}
.v-a4-contact {
  margin: 4px 0 0;
  font-size: 10.5px;
  color: #4b5563;
  overflow-wrap: anywhere;
}
.v-a4-sec {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.v-a4-h3 {
  margin: 0;
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: #111827;
  border-bottom: 1px solid #e5e7eb;
  padding-bottom: 2px;
}
.v-a4-p {
  margin: 0;
  font-size: 11.5px;
  line-height: 1.48;
  color: #1f2937;
}
.v-a4-kw-hit {
  background: #d1fae5;
  color: #065f46;
  padding: 0 3px;
  border-radius: 3px;
}
.v-a4-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin-top: 3px;
}
.v-a4-item-top {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  font-size: 11.5px;
  color: #111827;
}
.v-a4-sub {
  font-size: 11px;
  color: #4b5563;
}
.v-a4-ul {
  margin: 2px 0 0;
  padding-left: 16px;
  font-size: 11px;
  line-height: 1.45;
  color: #1f2937;
}
</style>
