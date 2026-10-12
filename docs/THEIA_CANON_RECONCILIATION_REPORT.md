# THEIA — SOURCE REVIEW AND CANON RECONCILIATION (12 October 2026)

**Primary evidence:** the original 403-page `Create MMO Story.pdf`, supplied by the user. **Story reference:** [THEIA_INTERWOVEN_CHRONICLE.md](THEIA_INTERWOVEN_CHRONICLE.md). **Short canon reference:** [THEIA_WORLD_BIBLE.md](THEIA_WORLD_BIBLE.md).

## Scope and verification method

The original conversation was reviewed from the rejected initial pitches through the 62-stage collaborative question process and subsequent long-form narrative/voice discussions. The first approximately 40 PDF pages contain abandoned pitches and **must not be imported as canon**. All 444 distinct numbered question identifiers were indexed, including **471 separately identifiable collaborative question-prompt occurrences** where numbers were reused.

Answers were associated with explicit user replies and later corrections, not form defaults or assistant recommendations. The previous 412-response candidate audit was tested against its assigned PDF-page neighbourhood; **411 matched**, and Q60 was manually verified on PDF p72 as the user’s explicit correction to **all six races**. Reused question variants and later overrides were then adjudicated. **418 numbered identifiers have a source-located answer or a confirmed later variant; 26 remain open/deferred.** These are identifier-level source-index counts, not a guarantee that every later unnumbered suggestion was approved.

The complete source register (approximately 367 KB) is available as `THEIA_FULL_SOURCE_VERIFIED_REGISTER.md` in the ChatGPT file workspace; it is **not yet in this GitHub repository**. The original PDF is also not in this repository. The repo contains a reproducible [question-extraction script](../tools/extract_theia_questions.py), the [compact decision recovery](THEIA_SOURCE_DECISION_RECOVERY.md), and the [verification status](THEIA_LINE_BY_LINE_VERIFICATION_STATUS.md).

## Exact open/deferred question identifiers

**Q179–183**, **Q194–197** (reissued geographical/access prompts), **Q212–219**, and **Q436–444**: 26 distinct identifiers. The original PDF also contains unanswered earlier variants of some reused numbers. The final group Q436–444 explicitly leaves the surviving original great magical order’s purpose, archives, survival method, Guardian relationship and the distribution/authenticity of ancient records unresolved.

## Important decisions restored from the original source

| Source | User-approved choice | Historical consequence |
|---|---|---|
| Q35 | **Direct ancestry** | Magical inheritance has genealogical roots; later rare empowered lineages are consistent |
| Q40 | **Hidden corruption of the seventh people's creation** | Nhal's subtle influence must be causally present in the creation project, not only in research centuries later |
| Q42 | **Divine empowerment** | Original exceptional bloodlines arise through the gods' deliberate acts |
| Q47 | **Mixed morality** | The Seven have divergent ethical characters, not uniformly benevolent intentions |
| Q55 → Q60 | Four races **superseded by six** | Six species and eight playable civilizations are the later model |
| Q309 | Keeper's knowledge + Ascendant's ambition | Their joint troll creation need not imply close friendship |
| Q315–317 (Stage 47) | Magical stability; unauthorized experimentation; shared halfling–troll resilience | Explains Keeper's distrust and troll/halfling magical ancestry |
| Q315–317 (Stage 49) | Different divine specialties; limitations through laws/understanding/consequences; gradual discovery | **Different prompts sharing the same numbers**; both sets matter |
| Q382 | Revised to **D**, all three | Halfling combat engineering, precision and defensive mastery |
| Q420 | Revised to **D**, all three | Founding divine teachings, occasional direct visits and mortal intermediaries |
| Q191–193 (reissued) | Almost complete separation; Shrouded Lands concept; combined PvP | Hostile faction divide and varied territorial warfare |
| Q198–200 (reissued) | Hybrid decay; yes to overextension; all three neutralization outcomes | Prevents permanent domination of the Shrouded Lands |
| Q201–202 | Moderate overextension; dynamic neutralization | Territory type, magical instability and activity determine outcomes |
| Q203–211 | Logistics, defensive NPC support, underdog incentives; siege limits/windows; shared upkeep; persistent ownership with seasonal objectives | Source-confirmed fairness principles, **not** final numerical balancing |
| Q427–435 | Three or four great independent interconnected orders, exactly one secret survivor | Institutional history must not invent the survivor's identity as approved canon |

**Original naming and design supersessions:** The Ascendant, Covenant, Dominion, Aelari, Vaelari, Korrath, Vorrath, The Unmade, Queen Seralyth, First Elder Dromak and First Archon Selrion. The Shaper's ambition remains a valid character flaw despite the seventh god's renaming.

## Narrative rewrite and historical causation

The [current interwoven chronicle](THEIA_INTERWOVEN_CHRONICLE.md) was substantially expanded and reordered to keep related histories together rather than isolate gods, races, hidden orders and modern politics in disconnected entries. Its continuous causal sequence now covers:

- Aethyr's **deliberate desperate First Sundering** and the Seven's independent awakening; the gods' mixed morality, emerging friendships, rivalries, creative philosophies and relationships to mortals.
- The six species, Keeper and Ascendant's cooperation/distrust during troll creation, divine empowerment of mortal bloodlines and the growth of complex independent magical institutions.
- The **three-or-four-order** Age of Wonders, knowledge sharing, Passage infrastructure and the social dependencies that made its destruction devastating.
- The creation of the exceptional seventh people and **proposed** preliminary Nhal influence during design, followed by stronger boundary contact through advanced research; hidden evidence manipulation and the Divine Schism.
- Keeper's **working** discovery of an intelligence outside creation, Guardian's protective dilemma and a **working** Great Betrayal/Second Sundering mechanism. The user has not ratified the exact actors, divine wartime sides or individual god fates.
- Guardians founded during Divine War; The Unmade rooted in Nhal-influenced ancient research; the **working** loss of Unmade contact after the sealing and renewed contact during the awakening.
- Distinct postwar survival of the dwarf and Korrath ancestral capitals, destruction of the original elven capital, migrations of all six peoples, halfling refuge among orcs, Covenant's human–dwarf treaty and Dominion's trial-based leadership.
- Aelari–Vaelari responsibility schism and Korrath–Vorrath religious exile, including Queen Seralyth, First Archon Selrion and First Elder Dromak; other proposed modern rulers remain marked as working names.
- Generations of resource warfare and naval expeditions, exceptional peace negotiations sabotaged by The Unmade, Guardian investigation and the expansion of hostile overseas outposts.
- Level-banded military, Guardian and Unmade incursions on both continents, the later Shrouded Lands PvP theatre and two permanently separate launch campaigns toward a working level-60 cap.

## Still open by deliberate design, not by failure to extract

The exact Great Betrayal and sealing machinery; which gods fought on which side and survived in which condition; the vanished seventh people's name, physical form and ultimate fate; the one original surviving magical order's purpose, method, archives and relationship to Guardians; exact NPC biographies beyond user-confirmed names; world-map coordinates and definitive PvP numerical balance; the final fate of Nhal and the Seven. The chronicle contains **working** dramatic interpretations of some of these to create a readable full saga; they must not be promoted to confirmed canon without a user decision.

## Repository decisions

- **Keep and update** `THEIA_WORLD_BIBLE.md` as the canonical constraint reference.
- **Keep and update** `THEIA_INTERWOVEN_CHRONICLE.md` as the primary continuous, creator-facing history.
- **Keep** `THEIA_DETAILED_LORE_ARCHIVE.md` as secondary detail/reference, with source correction notices.
- **Delete from current branch** the redundant `THEIA_TRUE_CHRONICLE.md`; its older text remains recoverable through Git history.
- **Add** the source decision recovery, verification status and this report, while preserving the 403-page PDF as the authority for disputed question wording.

**Audit integrity:** A full machine-indexed source pass and targeted manual collision/correction review have been completed. A literal human rereading and independent adjudication of all 403 pages word by word is not claimed. This distinction prevents overstating the evidence.

## Later creator clarification: the deliberate two-realm First Sundering (2026-10-12)

The user supplied an expanded **draft**, not final prose, explaining **why** Aethyr shattered itself. Aethyr and Nhal were supreme primordial essences before space, time, laws or realms. Their conflict of creation versus unmaking reached a stalemate. Aethyr realized that **imprisonment through separation**, rather than direct defeat, could protect future creation. Its voluntary shattering **created both the Physical Realm and the Void**, forced Nhal into the latter and its own shards into the former, and **deliberately formed the Veil as a final protective act**. This replaces earlier ambiguity about whether the Void existed before creation or the Veil was incidental. The World Bible, Detailed Lore Archive and Interwoven Chronicle were revised to express this causal sequence. Veil mechanics and the ultimate fate of Nhal remain open.

## Later locked decision — early divine relationships and creation order (2026-10-12)

The user selected an initial **philosophical disagreement** as the foundation of Guardian–Warrior's eventual close friendship, then approved the complete **discovery-first** model of mortal creation. Seeker and Shaper's intellectual partnership discovers the foundational principles of independent life but **deliberately postpones creating it because their knowledge is incomplete**. Guardian and Warrior take the research further to create elves and orcs (in that order). Shaper creates dwarves, Judge creates humans and Keeper creates halflings; Seeker remains an influential scholar/patron without creating one of these original six species. Ascendant has already awakened independently but **joins the other gods after all five peoples exist**. His collaboration with Keeper produces trolls as the sixth people; the seventh-people project follows much later and involves several gods. This new chronology is locked in the World Bible and written into the opening of the Interwoven Chronicle. Specific encounters/dialogue and the first six gods' precise awakening order remain open.

## 2026-10-12 confirmed troll-creation revision

The user explicitly rejected an earlier narrative draft that made trolls sound nearly unkillable and approved a different explanation for Ascendant's unauthorized modification. **Ascendant amplified the trolls' ancestral connection to divine essence**, hoping to be worshipped over other gods. The alteration helps explain the trolls' unusually strong religious traditions, while preserving their free will and mortality; it does not grant exceptional regeneration, instant evolution, guaranteed divine communication, superior magical reserves or combat power. Keeper's objection is an ethical and trust breach, not evidence that trolls are defective. Troll devotion later divides between Keeper and Ascendant, contributing to—but not mechanically causing—the Korrath–Vorrath religious conflict and exile. The old automatic-adaptation narrative is superseded. The World Bible, Detailed Lore Archive and Interwoven Chronicle have been revised accordingly.

## 2026-10-12 final-sealing decision: The Binding

The user selected **The Binding** as the historical name for the final containment and clarified that **Keeper knows with absolute certainty** the other gods will be bound indefinitely to maintaining the Shroud, **but intentionally does not tell them**. The other six believe they are donating a limited amount of essence. All Seven survive as distinct conscious essences, preserving the possibility of future reunification into a single essence to confront Nhal after emergence; this possible ending is not yet confirmed. The moral betrayal is concealed within The Binding, not its official event name. The earlier Guardian–Keeper infrastructure-commandeering betrayal and literal death of gods in the sealing are superseded as governing canon. The World Bible, Detailed Lore Archive and Interwoven Chronicle have been revised to reflect the decision.

## 2026-10-12 Anchor-destruction reconciliation (user locked)

The user approved the narrative/gameplay explanation that the **First Anchor** was not indestructible in itself but became too dangerous to destroy after entangling with a permanent breach in the Veil. The Seven's **Binding** and Shroud contained it instead. The Unmade lost the original seventh people's complete Anchor technology; their modern **Minor Resonators, Regional Anchors and Major Anchors** are progressively sophisticated but incomplete, destructible derivatives drawing limited influence through the original damaged connection. Player quests can safely dismantle these by disabling supports first where required. The Unmade must coordinate advanced networks to threaten the Shroud, avoiding an existential crisis for each minor quest. This is now documented in the World Bible, Detailed Lore Archive and Interwoven Chronicle. The First Anchor's eventual resolution, detailed implementation and possible divine reunification remain open.

## 2026-10-12 user decision: permanently weakened but continuously usable Passage gates

The user confirmed that after The Binding the ancient gates connected with the Shrouded Lands were **weakened but continuously operational at low capacity**. They did not require periodic magical alignments. This allowed The Unmade to move small numbers of agents, messages, artifacts and supplies for covert operations on both alliance continents, but prevented army-scale transport and open invasion. Passage transit remains within the Physical Realm, not direct access to Nhal across the Veil. Later restored major gateways can serve Shrouded Lands PvP while shipborne overseas military incursions continue separately. The World Bible, Detailed Lore Archive and Interwoven Chronicle now incorporate this historical and gameplay distinction. NPC passage of corrupted seventh people and exact gate specifications remain open.

## 2026-10-12 containment clarification: the Unmade's Shrouded epicentre

The user clarified that The Binding's Shroud **protects the world outside**, not that it eliminates corruption inside. Nhal's influence remains relentless around the First Anchor and ruined seventh-people homeland, making the Shrouded Lands the historical and continuing epicentre of The Unmade. This does not grant Nhal unrestricted incarnation, turn every transformed seventh person into an Unmade agent, or restore direct communication during the post-Binding Age of Silence. Surviving low-capacity gates permit covert infiltration across both continents. The World Bible, Detailed Lore Archive and Interwoven Chronicle have been aligned with this causal interpretation.

## 2026-10-12 locked biological compatibility rule and external Seventh diaspora

The user approved a **supersession of the immediately preceding general interbreeding rule**: of the seven original mortal species, **only The Seventh** can have children across species boundaries with any of the other six. The six original species do not ordinarily reproduce with one another across species, though Aelari/Vaelari and Korrath/Vorrath remain same-species cultural branches. This is a unique result of Ascendant's design, not a source of guaranteed exceptional magical or combat strength. Seventh scholars and survivors outside the Shroud could integrate into other civilizations through families over generations, concealing their ancestry through ordinary cultural assimilation, lost records and migrations. Distinct surviving enclaves inside the Shroud remain intact. Exact genetics, descendant fertility and Guardian founding genealogies are not fixed. The World Bible, Detailed Lore Archive, Interwoven Chronicle and decision recovery are updated to reflect the later user choice.

## 2026-10-12 canon lock: Guardians' founding and deliberate Seventh secrecy

The user approved the full Guardian origin developed after the unique Seventh cross-species rule. **During the Divine War, before The Binding**, surviving seventh-people investigators—some already abroad when the First Anchor activated, others escaping before the Shroud—formed a **multiracial secret coalition** with trusted scholars, witnesses and protectors of the other six peoples. They knew only that **an unknown influence was manipulating magical research**, not Nhal's identity. They preserved evidence, sought to prevent repeat catastrophes, and refused to entrust the whole truth to a single god or mortal institution.

The Guardians established cells and hidden repositories on both sides of what would become the Shroud. After The Binding, weakened but continuously low-capacity gates allowed limited contact, not mass evacuation. **Early Guardians intentionally concealed surviving Seventh enclaves** to protect them. Compartmentalization, lost archives, migration and intergenerational secrecy gradually caused **many later Guardians to forget the full truth themselves**. The Guardians are neither all Seventh nor hereditary; their recruits come from the six peoples and possibly unrecognized Seventh descendants. Their growing secrecy creates a thematic parallel with Keeper's withheld truth.

This resolves the earlier *proposed* Guardian ancestry and concealment as **confirmed user decisions**. The Seventh's small enclaves, the Unmade's Nhal-serving movement, the Guardians' multiracial mission and the separate secretly surviving original great magical order remain distinct. Exact founders, archives, senior knowledge and internal offices are still open. Updated files: World Bible, Interwoven Chronicle, Detailed Lore Archive and Source Decision Recovery.
