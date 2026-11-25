# Contributing to Vecta

Thank you for your interest in contributing to **Vecta**. Because Vecta aims to serve job seekers across every academic major and industry in Indonesia (and beyond), community contributions to the **14-domain skill taxonomy**, **portal adapters**, and **ATS templates** are especially valuable.

---

## 1. Expanding the Cross-Domain Skill Taxonomy (`shared/taxonomy/`)

Skill dictionaries live in `shared/taxonomy/01_*.json` through `14_*.json`.

Each entry follows this schema:

```json
{
  "id": "fin.brevet_ab",
  "canonical_name": "Perpajakan (Brevet A/B)",
  "domain": "ekonomi_akuntansi_pajak",
  "category": "core_hard",
  "parent_id": "fin.perpajakan",
  "aliases": ["Brevet A", "Brevet B", "Brevet A/B", "Tax Brevet"],
  "requires_domain_context": false
}
```

Guidelines for adding skills:
- Use clear canonical names recognized by Indonesian recruiters and global ATS systems.
- Set `"requires_domain_context": true` for ambiguous short terms or cross-domain acronyms (such as `QA`, `Audit`, `CRM`, `Go`, `Rust`) so they do not trigger false positives in unrelated fields.
- Add a real-world job fixture to `crates/vecta-taxonomy/tests/benchmark.rs` when introducing a new specialty.
- Verify the precision and recall gates before opening a PR:
  ```bash
  cargo test -p vecta-taxonomy
  ```

---

## 2. Code & UI Contributions

1. Fork the repository and create a topic branch from `Development` (or `main`).
2. Follow the CSS design tokens in `apps/client/src/styles/tokens.css` for all Vue 3 UI changes.
3. Run all verification checks locally before submitting a pull request:
   ```bash
   cargo test --workspace
   cd apps/client && npm run typecheck && npm test && npm run build
   ```
4. Keep commits focused using Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`).
