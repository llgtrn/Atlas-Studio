# Ops Product Display Language Contract

Status: **ACTIVE OPS PRODUCTION CONTRACT — v1**

This contract defines how every Chronica-related Ops product presents human-facing activity, status, actions, evidence and multilingual copy so that an Ops surface can later compose into Chronica without importing a second UI language or design system.

The shared architecture owner is:

```text
docs/architecture/foundation/product-language.md
```

The governing principle is:

```text
Ops semantics/runtime output
-> reusable ActivityProjection
-> shared post/artifact/status/action grammar
-> localized human copy
```

Ops may have a distinct product identity. It must not invent a competing display ontology.

---

## 1. One interaction grammar across Ops

TradingOps, BnbOps, HelpdeskOps, CustomerOps, FinOps and future Ops products SHOULD converge on the same reusable presentation grammar:

```text
ActivityPost
Artifact
Status
Actions
Evidence
Thread
Workspace
```

Domain specialization belongs in the artifact/body and available intents, not in a completely different shell.

Examples:

```text
TradingOps: order filled
BnbOps: reservation confirmed
HelpdeskOps: guest replied
FinOps: reconciliation mismatch detected
MachineOps: alarm entered warning state
```

All may be rendered by the same activity/post family.

---

## 2. ActivityProjection is the integration seam

Every Ops domain SHOULD provide a mapping from meaningful local events/observations/execution states into the shared ActivityProjection shape.

Minimum portable fields:

```text
activityId
sourceRefs[]
subjectRef
actorRef
verb
objectRef
occurredAt
observedAt
classification
status
headlineKey
bodyKey
messageArgs
artifact
actions[]
evidenceRefs[]
threadRef
scope
sensitivity
```

A field may be null when semantically absent. It must not be fabricated merely to fill a card.

The integration target is:

```text
Ops local projection
-> ActivityProjection-compatible output
-> Chronica shared feed/artifact components
```

NOT:

```text
Ops local shell
-> iframe/embed/duplicate shell inside Chronica
```

---

## 3. Post-like representation is projection only

An Ops activity post is not canonical storage.

Forbidden:

```text
post table becomes authoritative business state
post text becomes evidence
post status becomes authority
translated copy becomes canonical fact
```

Required:

```text
post references real source state/events/evidence
```

The UI may be regenerated from those references.

---

## 4. Minimalist white visual baseline

All Ops products SHOULD use the same visual direction as Chronica:

```text
light-first
white / near-white surfaces
neutral borders
minimal shadows
restrained radius
compact but breathable spacing
one primary accent
semantic status colors only
content before decoration
```

Avoid permanent donor visual identity unless deliberately exposed as provider branding.

Do not fork a separate design-token universe per Ops product.

Ops may customize product branding at the shell edge, but shared semantic status/risk/authority meanings must remain visually consistent.

---

## 5. Multilingual contract

Reusable user-facing strings MUST be locale-key based.

Preferred:

```text
headlineKey = "activity.order.filled"
messageArgs = { symbol, amount, price, currency }
```

Avoid constructing portable activity text as a final English sentence in domain/runtime code.

Required language design must support at least:

```text
English
Japanese
Vietnamese
Simplified Chinese
Traditional Chinese
```

Additional locales remain allowed.

Locale formatting applies to dates, time zones, numbers, currency and units without changing canonical values.

---

## 6. Stable keys, not donor/component keys

Translation keys represent product meaning.

Prefer:

```text
activity.order.filled
activity.reservation.cancelled
activity.conversation.reply_received
status.execution.pending_approval
action.open_evidence
```

Avoid:

```text
BinanceOrderFilled
ChatwootReplyRow
ERPNextInvoiceState
PostCard.line2
```

Provider/donor names may appear in boundary metadata or explicit provider labels, not as the universal product-language key.

---

## 7. Provider vocabulary stays at the boundary

Example:

```text
provider operation: Binance GET /api/v3/aggTrades
Ops semantic:       market.trades
product label:      Trades
```

or:

```text
donor term:     resolve_ticket
Ops semantic:   conversation.close
product label:  Close conversation
```

Do not leak raw provider endpoint vocabulary into navigation or reusable status/action language unless the user explicitly needs provider detail.

---

## 8. RECORD / ANALYZE / ACT display classification

Every portable activity projection MUST preserve its epistemic/action class:

```text
RECORD
ANALYZE
ACT
SYSTEM
```

The shared post shell must not make these look equivalent.

Examples:

```text
forecast changed        -> ANALYZE
reservation observed    -> RECORD
order submitted         -> ACT
provider disconnected   -> SYSTEM
```

Prediction is not fact. Requested is not executed. Provider success is not automatically canonical success.

---

## 9. Action controls are intents only

Ops UI actions may expose:

```text
Reply
Approve
Reject
Retry
Run
Cancel
Open
Inspect
```

These are UI intents and navigation affordances.

They MUST flow through the Ops/Chronica authority and execution path where effectful.

Button presence/absence is not authorization.

---

## 10. Shared artifact grammar

Domain depth SHOULD be expressed through typed artifacts attached to the reusable post.

Examples:

```text
TradingOps -> order / position / market artifact
BnbOps -> reservation / property / pricing artifact
HelpdeskOps -> conversation / guest / SLA artifact
FinOps -> account / reconciliation / payment-request artifact
```

Artifacts may open specialized workspaces while preserving source refs, scope, authority and evidence continuity.

---

## 11. Donor UI intake

When deep-forking a donor UI:

```text
clone exact donor
-> preserve baseline
-> census screens/components/copy/i18n
-> classify useful interaction patterns
-> map to shared Activity/Post/Artifact grammar
-> migrate useful flows
-> retire duplicate shell/design/i18n ownership
```

Do NOT preserve a donor component merely because it visually exists.

Preserve engineering value; dissolve competing product ontology.

---

## 12. Ops manifest expectations

An Ops repository implementing a user-facing surface SHOULD be able to report:

```text
display_language:
  contract_version: 1
  activity_projection: implemented | partial | absent
  shared_design_tokens: implemented | partial | absent
  locale_catalog: implemented | partial | absent
  hardcoded_copy_debt: <count or evidence ref>
  donor_term_leak_debt: <count or evidence ref>
  provider_term_leak_debt: <count or evidence ref>
  chronica_composition_ready: true | false
```

This evidence is descriptive. It does not by itself prove semantic absorption.

---

## 13. TradingOps example

A Binance fill should not require a TradingOps-only card type.

Preferred mapping:

```text
sourceRefs:
  provider execution evidence + order resource
verb:
  execution.order_filled
classification:
  ACT
status:
  success
headlineKey:
  activity.order.filled
artifact:
  OrderArtifact
providerLabel:
  Binance Spot
```

The same renderer can later run inside Chronica with no semantic rewrite.

---

## 14. Composition-ready gate

Before calling an Ops UI composition-ready, verify:

```text
1. reusable activity mapping exists for meaningful changes
2. source/evidence references survive projection
3. effectful actions still route through authority/execution
4. reusable copy uses locale keys
5. domain/provider names do not become universal UI ontology
6. shared design tokens are used for semantic state
7. donor shell ownership is not required for the product to function
8. specialized workspaces can open from the shared activity/artifact model
```

If these are true, integration should mostly be composition and routing rather than UI reimplementation.

---

## Final rule

> **OPS MAY KEEP PRODUCT IDENTITY, BUT IT MUST SPEAK THE SAME HUMAN-FACING GRAMMAR: ONE REUSABLE POST/ACTIVITY MODEL, ONE ARTIFACT/ACTION/EVIDENCE LANGUAGE, ONE LOCALIZATION MODEL, AND ONE MINIMALIST SEMANTIC DESIGN SYSTEM.**
