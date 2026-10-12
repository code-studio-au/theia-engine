> **SOURCE REVIEW COMPLETE (12 October 2026):** Read [THEIA_CANON_RECONCILIATION_REPORT.md](THEIA_CANON_RECONCILIATION_REPORT.md) for the 403-page source-index methodology, 471 separately indexed question prompt occurrences, 418 source-located numbered identifiers, 26 deliberately open/deferred identifiers, and corrections propagated into the current World Bible and chronicle. The complete 367 KB original-question evidence register remains available as a ChatGPT download but is not yet committed to GitHub.

> **VERIFICATION UPDATE (2026-10-12):** See [THEIA_LINE_BY_LINE_VERIFICATION_STATUS.md](THEIA_LINE_BY_LINE_VERIFICATION_STATUS.md) for the source audit covering all 444 question slots: 412 source-located responses, 2 newly source-verified answers and 30 unanswered/deferred original numbered entries. A complete 336 KB Markdown question-by-question evidence register has been generated in the ChatGPT file workspace but is **not yet committed to GitHub**. Full manual adjudication of all later corrections remains open.

# Theia documentation — source of truth and reading order

## Current documents (after the original 403-page conversation export was supplied)

1. **[THEIA_WORLD_BIBLE.md](THEIA_WORLD_BIBLE.md)** — short canonical reference. Where a user-confirmed decision conflicts with an assistant proposal, the decision wins.
2. **[THEIA_INTERWOVEN_CHRONICLE.md](THEIA_INTERWOVEN_CHRONICLE.md)** — current creator-facing chronological narrative, expanded with divine relationships, the vanished seventh people, Keeper/Ascendant conflict, Nhal, ancient institutions, Second Sundering, racial migrations, the Guardians and Unmade, Shattered Delegation, modern leaders and overseas incursions. It contains proposed connective history that is NOT automatically ratified.
3. **[THEIA_DETAILED_LORE_ARCHIVE.md](THEIA_DETAILED_LORE_ARCHIVE.md)** — supporting worldbuilding archive, cultural systems, world regions, institutions and late-stage question answers.
4. **[THEIA_SOURCE_DECISION_RECOVERY.md](THEIA_SOURCE_DECISION_RECOVERY.md)** — source-audit summary for original Q1–435, including corrections and later renames.

The earlier duplicate **THEIA_TRUE_CHRONICLE.md** was removed from the current branch to avoid two competing primary narratives. Its content remains recoverable from Git history.

## Original primary source

**Create MMO Story.pdf** — 403-page export of the original conversation, provided in ChatGPT on 12 October 2026. This PDF is the ultimate evidence for user decisions. It is **not yet stored in this repository**. The source contains rejected alternate story proposals before the collaborative Stage 1 questions begin around page 41; these rejected concepts must never be silently imported as canon.

To generate the full question-prompt index locally after placing the PDF in your checkout:

```bash
python tools/extract_theia_questions.py "Create MMO Story.pdf"
```

This produces `docs/THEIA_ORIGINAL_QUESTIONS_Q001-Q444.md`. The generated file records original question wording and options but **does not automatically infer which answer the user selected**. The extraction script is committed; the full generated 444-question file and original PDF are not yet committed.

## Editorial guardrails

- **CONFIRMED** — explicitly selected by the user, including later corrections and names.
- **WORKING** — coherent narrative proposal; may be adopted, revised or rejected.
- **OPEN** — no final decision.
- **SUPERSEDED** — old name, earlier draft or previous selection replaced by later user instruction.

The question numbers were reused around Q297–317, so identify questions by stage and source page as well as number. Stage 62 Q436–444 received no submitted answers. The creator narrative's detailed Second Sundering machinery, original surviving great order identity and fate of individual gods are still working/open.

## Remaining archival work

The recovered decision file is a *summary*, not a complete verified answer register. The remaining high-effort task is to compare every original Q1–435 prompt with the **following user response and all later corrections**, record exact source pages and status, then reconcile each confirmed consequence into the World Bible and narrative. This must not be described as finished until the audit is actually performed.

The story's long-form draft is deliberately separate from the authoritative decision ledger so it can become increasingly novel-like without erasing provenance.
