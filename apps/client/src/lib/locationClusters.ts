export interface LocationClusterOption {
  id: string;
  label: string;
  group: string;
  aliases: string[];
}

export const LOCATION_CLUSTER_OPTIONS: LocationClusterOption[] = [
  // 1. Fleksibel & Remote
  {
    id: "remote_indonesia",
    label: "Remote (Indonesia / WFA)",
    group: "Fleksibel & Remote",
    aliases: ["remote", "wfh", "wfa", "work from home", "work from anywhere", "anywhere in indonesia", "remote id"],
  },
  {
    id: "luar_negeri_remote",
    label: "Remote (Global / Lintas Negara)",
    group: "Fleksibel & Remote",
    aliases: ["remote globally", "worldwide", "global", "apac", "sea", "international remote", "anywhere"],
  },

  // 2. Jabodetabek & Banten
  {
    id: "jakarta",
    label: "DKI Jakarta",
    group: "Jabodetabek & Banten",
    aliases: ["jakarta", "jaksel", "jakpus", "jakbar", "jaktim", "jakut", "scbd", "sudirman", "kuningan", "senopati", "thamrin", "kemang", "pik", "pluit", "kelapa gading", "tb simatupang"],
  },
  {
    id: "bodetabek",
    label: "Bodetabek (Bogor, Depok, Tangerang, Bekasi)",
    group: "Jabodetabek & Banten",
    aliases: ["bogor", "cibinong", "sentul", "depok", "tangerang", "tangsel", "tangerang selatan", "bsd", "alam sutera", "gading serpong", "karawaci", "bekasi", "cikarang", "tambun", "jabodetabek"],
  },
  {
    id: "banten_lainnya",
    label: "Banten Lainnya (Serang, Cilegon, Lebak)",
    group: "Jabodetabek & Banten",
    aliases: ["banten", "serang", "cilegon", "lebak", "pandeglang", "merak", "anyer"],
  },

  // 3. Jawa Barat
  {
    id: "bandung_raya",
    label: "Bandung Raya & Cimahi",
    group: "Jawa Barat",
    aliases: ["bandung", "cimahi", "sumedang", "jatinangor", "soreang", "lembang", "padalarang"],
  },
  {
    id: "jabar_lainnya",
    label: "Jawa Barat Lainnya (Karawang, Cirebon, Sukabumi)",
    group: "Jawa Barat",
    aliases: ["karawang", "purwakarta", "subang", "cirebon", "indramayu", "majalengka", "tasikmalaya", "garut", "sukabumi", "cianjur", "banjar", "jawa barat", "west java"],
  },

  // 4. Jawa Tengah & DIY
  {
    id: "yogyakarta",
    label: "DI Yogyakarta (Kota, Sleman, Bantul)",
    group: "Jawa Tengah & DIY",
    aliases: ["yogyakarta", "jogja", "diy", "sleman", "bantul", "kulon progo", "gunungkidul"],
  },
  {
    id: "semarang_raya",
    label: "Semarang & Pantura Jateng",
    group: "Jawa Tengah & DIY",
    aliases: ["semarang", "ungaran", "kendal", "salatiga", "demak", "kudus", "jepara", "pati", "pekalongan", "tegal", "batang", "brebes", "pemalang"],
  },
  {
    id: "solo_jateng",
    label: "Solo Raya & Jateng Selatan",
    group: "Jawa Tengah & DIY",
    aliases: ["solo", "surakarta", "sukoharjo", "karanganyar", "boyolali", "klaten", "sragen", "wonogiri", "purwokerto", "banyumas", "cilacap", "magelang", "kebumen", "jawa tengah", "central java"],
  },

  // 5. Jawa Timur
  {
    id: "surabaya_raya",
    label: "Surabaya, Sidoarjo, Gresik & Mojokerto",
    group: "Jawa Timur",
    aliases: ["surabaya", "sidoarjo", "gresik", "mojokerto", "lamongan", "bangkalan", "madura"],
  },
  {
    id: "malang_jatim",
    label: "Malang & Jawa Timur Lainnya",
    group: "Jawa Timur",
    aliases: ["malang", "batu", "pasuruan", "probolinggo", "kediri", "blitar", "tulungagung", "jember", "banyuwangi", "madiun", "nganjuk", "bojonegoro", "tuban", "jawa timur", "east java"],
  },

  // 6. Bali & Nusa Tenggara
  {
    id: "bali",
    label: "Bali (Denpasar, Badung, Ubud)",
    group: "Bali & Nusa Tenggara",
    aliases: ["bali", "denpasar", "badung", "kuta", "seminyak", "canggu", "ubud", "jimbaran", "nusa dua", "gianyar", "tabanan", "singaraja", "buleleng"],
  },
  {
    id: "ntb_ntt",
    label: "NTB & NTT (Lombok, Kupang, Labuan Bajo)",
    group: "Bali & Nusa Tenggara",
    aliases: ["lombok", "mataram", "sumbawa", "bima", "ntb", "nusa tenggara barat", "ntt", "kupang", "flores", "labuan bajo", "maumere", "ende", "sumba", "nusa tenggara timur"],
  },

  // 7. Sumatera
  {
    id: "medan_sumut",
    label: "Medan, Sumatera Utara & Aceh",
    group: "Sumatera",
    aliases: ["medan", "deli serdang", "binjai", "pematangsiantar", "samosir", "sibolga", "sumatera utara", "sumut", "aceh", "banda aceh", "lhokseumawe"],
  },
  {
    id: "batam_kepri",
    label: "Batam, Bintan & Kep. Riau",
    group: "Sumatera",
    aliases: ["batam", "bintan", "tanjung pinang", "karimun", "kepri", "kepulauan riau", "natuna"],
  },
  {
    id: "pekanbaru_riau",
    label: "Pekanbaru, Padang & Sumatera Tengah",
    group: "Sumatera",
    aliases: ["pekanbaru", "dumai", "riau", "bengkalis", "siak", "padang", "bukittinggi", "sumbar", "sumatera barat", "jambi", "kerinci", "payakumbuh"],
  },
  {
    id: "palembang_sumbagsel",
    label: "Palembang, Lampung, Bengkulu & Babel",
    group: "Sumatera",
    aliases: ["palembang", "muara enim", "prabumulih", "sumsel", "sumatera selatan", "bandar lampung", "lampung", "bengkulu", "bangka", "belitung", "pangkalpinang", "sumatera", "sumatra"],
  },

  // 8. Kalimantan
  {
    id: "balikpapan_ikn",
    label: "Balikpapan, Samarinda, IKN & Kaltara",
    group: "Kalimantan",
    aliases: ["balikpapan", "samarinda", "ikn", "nusantara", "penajam", "bontang", "sangatta", "kutai", "berau", "tarakan", "kaltim", "kalimantan timur", "kaltara", "kalimantan utara"],
  },
  {
    id: "banjarmasin_kalsel",
    label: "Banjarmasin, Palangkaraya (Kalsel & Kalteng)",
    group: "Kalimantan",
    aliases: ["banjarmasin", "banjarbaru", "kalsel", "kalimantan selatan", "tabalong", "kotabaru", "palangkaraya", "sampit", "pangkalan bun", "kalteng", "kalimantan tengah"],
  },
  {
    id: "pontianak_kalbar",
    label: "Pontianak & Kalimantan Barat",
    group: "Kalimantan",
    aliases: ["pontianak", "singkawang", "ketapang", "sintang", "kalbar", "kalimantan barat", "kalimantan"],
  },

  // 9. Sulawesi
  {
    id: "makassar_sulsel",
    label: "Makassar, Palu, Kendari (Sulsel/Teng/Tra/Bar)",
    group: "Sulawesi",
    aliases: ["makassar", "gowa", "maros", "parepare", "sulsel", "sulawesi selatan", "palu", "morowali", "luwuk", "sulteng", "sulawesi tengah", "kendari", "kolaka", "sultra", "sulawesi tenggara", "mamuju", "sulbar", "sulawesi barat"],
  },
  {
    id: "manado_sulut",
    label: "Manado, Gorontalo & Sulawesi Utara",
    group: "Sulawesi",
    aliases: ["manado", "bitung", "tomohon", "minahasa", "sulut", "sulawesi utara", "gorontalo", "sulawesi"],
  },

  // 10. Maluku & Papua
  {
    id: "maluku_papua",
    label: "Maluku & Seluruh Papua",
    group: "Maluku & Papua",
    aliases: ["ambon", "maluku", "ternate", "halmahera", "weda", "maluku utara", "malut", "jayapura", "sorong", "manokwari", "timika", "mimika", "nabire", "merauke", "biak", "papua", "papua barat", "papua tengah", "papua selatan", "papua pegunungan"],
  },

  // 11. Internasional
  {
    id: "singapura",
    label: "Singapura",
    group: "Internasional",
    aliases: ["singapore", "singapura", "sg"],
  },
  {
    id: "malaysia",
    label: "Malaysia (KL, Penang, Johor)",
    group: "Internasional",
    aliases: ["malaysia", "kuala lumpur", "penang", "johor", "selangor", "cyberjaya"],
  },
  {
    id: "jepang_asia_timur",
    label: "Jepang, Korea & Asia Timur",
    group: "Internasional",
    aliases: ["japan", "jepang", "tokyo", "osaka", "korea", "seoul", "taiwan", "taipei", "hong kong"],
  },
  {
    id: "australia_nz",
    label: "Australia & Selandia Baru",
    group: "Internasional",
    aliases: ["australia", "sydney", "melbourne", "brisbane", "perth", "new zealand", "auckland"],
  },
  {
    id: "eropa_amerika",
    label: "Eropa, UK, Timur Tengah & Amerika",
    group: "Internasional",
    aliases: ["europe", "eropa", "uk", "london", "germany", "berlin", "netherlands", "amsterdam", "dubai", "uae", "usa", "united states", "canada", "amerika"],
  },

  // 12. Umum
  {
    id: "indonesia_lainnya",
    label: "Indonesia (Umum / Kota Lainnya)",
    group: "Umum",
    aliases: ["indonesia", "nasional", "on-site indonesia", "lainnya"],
  },
];

const LEGACY_CLUSTER_LABELS: Record<string, string> = {
  jabodetabek: "Jabodetabek (Semua)",
  bandung_jabar: "Jawa Barat (Semua)",
  surabaya_jatim: "Jawa Timur (Semua)",
  jogja_jateng: "Jateng & DIY (Semua)",
  bali_ntt: "Bali & Nusa Tenggara",
  sumatera: "Sumatera (Semua)",
  kalimantan: "Kalimantan (Semua)",
  sulawesi_timur: "Sulawesi & Indonesia Timur",
  luar_negeri: "Luar Negeri (Semua)",
  indonesia: "Indonesia (Umum)",
};

const CLUSTER_LABEL_MAP = new Map<string, string>(
  LOCATION_CLUSTER_OPTIONS.map((item) => [item.id, item.label]),
);

export function formatLocationClusterLabel(slug: string): string {
  const clean = slug.trim().toLowerCase();
  return CLUSTER_LABEL_MAP.get(clean) ?? LEGACY_CLUSTER_LABELS[clean] ?? slug;
}

const SHORT_CLUSTER_LABELS: Record<string, string> = {
  remote_indonesia: "Remote ID",
  luar_negeri_remote: "Remote Global",
};

export function formatLocationClusterShortLabel(slug: string): string {
  const clean = slug.trim().toLowerCase();
  if (SHORT_CLUSTER_LABELS[clean]) {
    return SHORT_CLUSTER_LABELS[clean];
  }
  const full = formatLocationClusterLabel(slug);
  return full.replace(/\s*\([^)]*\)\s*/g, "").trim() || full;
}

export function groupLocationClusters(
  items: LocationClusterOption[],
): Array<{ group: string; items: LocationClusterOption[] }> {
  const grouped = new Map<string, LocationClusterOption[]>();
  for (const item of items) {
    const arr = grouped.get(item.group);
    if (arr) {
      arr.push(item);
    } else {
      grouped.set(item.group, [item]);
    }
  }
  return Array.from(grouped.entries()).map(([group, groupItems]) => ({
    group,
    items: groupItems,
  }));
}

export function searchLocationClusters(
  query: string,
): Array<{ group: string; items: LocationClusterOption[] }> {
  const q = query.trim().toLowerCase();
  if (!q) {
    return groupLocationClusters(LOCATION_CLUSTER_OPTIONS);
  }
  const filtered = LOCATION_CLUSTER_OPTIONS.filter(
    (item) =>
      item.label.toLowerCase().includes(q) ||
      item.group.toLowerCase().includes(q) ||
      item.id.toLowerCase().includes(q) ||
      item.aliases.some((alias) => alias.includes(q)),
  );
  return groupLocationClusters(filtered);
}
