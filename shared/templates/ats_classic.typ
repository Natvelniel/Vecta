// Vecta ATS-Friendly Single-Column Classic CV Template (Typst)
// Guaranteed linear text extraction order for Applicant Tracking Systems.

#let data = json(bytes(sys.inputs.at("cv_json")))

#set page(
  paper: "a4",
  margin: (x: 1.6cm, y: 1.5cm),
)

#set text(
  font: ("Roboto", "Liberation Sans", "Inter", "DejaVu Sans", "Arial"),
  size: 10pt,
  fill: rgb("#111827"),
)

#set par(
  leading: 0.58em,
  justify: false,
)

#let section-heading(title) = {
  v(8pt)
  text(size: 10.5pt, weight: "bold", tracking: 0.4pt, fill: rgb("#0F172A"))[#upper(title)]
  v(-5pt)
  line(length: 100%, stroke: 0.75pt + rgb("#CBD5E1"))
  v(4pt)
}

// Header
#align(left)[
  #text(size: 18pt, weight: "bold", fill: rgb("#0F172A"))[#data.full_name]
  #if data.headline != "" {
    linebreak()
    text(size: 10.5pt, weight: "medium", fill: rgb("#334155"))[#data.headline]
  }
  #let contacts = (
    data.email,
    data.phone,
    data.location,
    data.linkedin_url,
    data.portfolio_url,
  ).filter(x => x != "")
  #if contacts.len() > 0 {
    linebreak()
    text(size: 9pt, fill: rgb("#475569"))[#contacts.join("  |  ")]
  }
]

// Professional Summary
#if data.summary != "" {
  section-heading(data.labels.summary)
  text(size: 9.5pt)[#data.summary]
}

// Tailored Skills (Matched job skills prioritized first)
#if data.skills.len() > 0 {
  section-heading(data.labels.skills)
  text(size: 9.5pt)[#data.skills.join("  •  ")]
}

// Work Experience
#if data.experiences.len() > 0 {
  section-heading(data.labels.experience)
  for exp in data.experiences {
    grid(
      columns: (1fr, auto),
      gutter: 8pt,
      [#text(weight: "bold")[#exp.role] #if exp.company != "" [ - #exp.company]],
      [#text(size: 9pt, fill: rgb("#475569"))[#exp.period]],
    )
    if exp.location != "" {
      text(size: 8.8pt, fill: rgb("#64748B"))[#exp.location]
      linebreak()
    }
    if exp.bullets.len() > 0 {
      for b in exp.bullets {
        pad(left: 8pt, top: 1pt, bottom: 1pt)[• #b]
      }
    }
    v(4pt)
  }
}

// Projects / Achievements
#if data.projects.len() > 0 {
  section-heading(data.labels.projects)
  for proj in data.projects {
    grid(
      columns: (1fr, auto),
      gutter: 8pt,
      [#text(weight: "bold")[#proj.name]],
      [#text(size: 9pt, fill: rgb("#475569"))[#proj.period]],
    )
    if proj.description != "" {
      pad(left: 8pt, top: 1pt)[#proj.description]
    }
    v(3pt)
  }
}

// Education
#if data.education.len() > 0 {
  section-heading(data.labels.education)
  for edu in data.education {
    grid(
      columns: (1fr, auto),
      gutter: 8pt,
      [#text(weight: "bold")[#edu.institution] #if edu.degree != "" [ - #edu.degree]],
      [#text(size: 9pt, fill: rgb("#475569"))[#edu.period]],
    )
    if edu.notes != "" {
      text(size: 9pt, fill: rgb("#475569"))[#edu.notes]
      linebreak()
    }
    v(3pt)
  }
}

// Certifications & Languages
#if data.certifications.len() > 0 or data.languages.len() > 0 {
  section-heading(data.labels.additional)
  if data.certifications.len() > 0 {
    [#text(weight: "bold")[#data.labels.certifications:] #data.certifications.join(", ")]
    linebreak()
  }
  if data.languages.len() > 0 {
    [#text(weight: "bold")[#data.labels.languages:] #data.languages.join(", ")]
  }
}
