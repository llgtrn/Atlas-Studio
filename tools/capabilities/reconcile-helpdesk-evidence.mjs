#!/usr/bin/env node
// reconcile-helpdesk-evidence.mjs — repair the 59 DANGLING verify-impl-evidence rows left by the
// chronica-helpdesk de-fork (the deleted crates/chronica-erp/src/inbox/*.rs files). Idempotent.
//
// The de-fork removed the inert Chatwoot pure-logic inbox modules; their impl_evidence rows still pointed at
// the deleted files, so `node tools/capabilities/verify-impl-evidence.mjs` failed (exit 1). This re-homes the
// evidence:
//   (A) RE-POINT — the 40 caps that ARE real production now (chronica-helpdesk store + chronica-api
//       helpdesk_* routes, proven by served-by route tests): rewrite impl_evidence to the live
//       file+symbol, keep status='verified', moves_money=0 (every helpdesk cap is config/record, NEVER money).
//   (B) DOWNGRADE — the 19 caps with NO real production equivalent (bots/attachments/inbox-registry/campaign/
//       clone/contact-search/import-export/bulk-contact/delete-conversation): DELETE the dangling evidence
//       row and drop status to 'unimplemented' with a blocker. Truth, not an inflated verified count.
//
// HARD GUARD (mirrors verify-impl-evidence + record-verified): every re-point is checked against REAL source
// BEFORE any write — the test_file must contain `fn <test_symbol>` and the impl_file must contain each
// impl_symbol. If ANY re-point fails its guard, the script ABORTS and writes NOTHING (all-or-nothing), so the
// caps.db can never be left half-applied or pointing at a missing symbol. NEVER flips a money flag.
//
// Run: node tools/capabilities/reconcile-helpdesk-evidence.mjs
import Database from 'better-sqlite3'
import { readFileSync, existsSync } from 'node:fs'
import { CAPABILITIES_DB } from '../_paths.mjs'

// ── (A) re-points: cap -> the REAL live code + served-by route test that proves it (all moves_money=0). ──
const REPOINTS = [
  { key: "agent-knowledge.create_conversation", test_file: "crates/chronica-api/tests/helpdesk_route.rs", test_symbol: "helpdesk_create_list_get_reply_list_happy_path", impl_file: "crates/chronica-api/src/routes/helpdesk.rs", impl_symbols: "create_thread", donor: "chatwoot" },
  { key: "agent-knowledge.list_conversations", test_file: "crates/chronica-api/tests/helpdesk_route.rs", test_symbol: "helpdesk_ordering", impl_file: "crates/chronica-api/src/routes/helpdesk.rs", impl_symbols: "list_threads", donor: "chatwoot" },
  { key: "agent-knowledge.show_conversation", test_file: "crates/chronica-api/tests/helpdesk_route.rs", test_symbol: "helpdesk_create_list_get_reply_list_happy_path", impl_file: "crates/chronica-api/src/routes/helpdesk.rs", impl_symbols: "get_thread", donor: "chatwoot" },
  { key: "agent-knowledge.toggle_conversation_status", test_file: "crates/chronica-api/tests/helpdesk_route.rs", test_symbol: "helpdesk_status_workflow_and_filter", impl_file: "crates/chronica-api/src/routes/helpdesk.rs", impl_symbols: "set_status", donor: "chatwoot" },
  { key: "agent-knowledge.set_conversation_priority", test_file: "crates/chronica-api/tests/helpdesk_route.rs", test_symbol: "helpdesk_priority_workflow", impl_file: "crates/chronica-api/src/routes/helpdesk.rs", impl_symbols: "set_priority", donor: "chatwoot" },
  { key: "agent-knowledge.mute_conversation", test_file: "crates/chronica-api/tests/helpdesk_mutes_route.rs", test_symbol: "mute_lifecycle_idempotent", impl_file: "crates/chronica-api/src/routes/helpdesk_mutes.rs", impl_symbols: "mute_thread, unmute_thread", donor: "chatwoot" },
  { key: "agent-knowledge.set_conversation_attributes", test_file: "crates/chronica-api/tests/helpdesk_custom_attribute_values_route.rs", test_symbol: "thread_attribute_values_set_get", impl_file: "crates/chronica-api/src/routes/helpdesk_custom_attributes.rs", impl_symbols: "set_thread_attributes, get_thread_attributes", donor: "chatwoot" },
  { key: "agent-knowledge.assign_conversation", test_file: "crates/chronica-api/tests/helpdesk_route.rs", test_symbol: "helpdesk_assign_and_unassign", impl_file: "crates/chronica-api/src/routes/helpdesk.rs", impl_symbols: "assign", donor: "chatwoot" },
  { key: "agent-knowledge.create_support_agent", test_file: "crates/chronica-api/tests/helpdesk_agents_route.rs", test_symbol: "agent_crud_happy_path_grants_nothing", impl_file: "crates/chronica-api/src/routes/helpdesk_agents.rs", impl_symbols: "create_agent", donor: "chatwoot" },
  { key: "agent-knowledge.list_support_agents", test_file: "crates/chronica-api/tests/helpdesk_agents_route.rs", test_symbol: "agent_crud_happy_path_grants_nothing", impl_file: "crates/chronica-api/src/routes/helpdesk_agents.rs", impl_symbols: "list_agents, get_agent", donor: "chatwoot" },
  { key: "agent-knowledge.update_support_agent", test_file: "crates/chronica-api/tests/helpdesk_agents_route.rs", test_symbol: "agent_crud_happy_path_grants_nothing", impl_file: "crates/chronica-api/src/routes/helpdesk_agents.rs", impl_symbols: "update_agent", donor: "chatwoot" },
  { key: "agent-knowledge.delete_support_agent", test_file: "crates/chronica-api/tests/helpdesk_agents_route.rs", test_symbol: "agent_crud_happy_path_grants_nothing", impl_file: "crates/chronica-api/src/routes/helpdesk_agents.rs", impl_symbols: "delete_agent", donor: "chatwoot" },
  { key: "agent-knowledge.get_conversation_messages", test_file: "crates/chronica-api/tests/helpdesk_route.rs", test_symbol: "helpdesk_create_list_get_reply_list_happy_path", impl_file: "crates/chronica-api/src/routes/helpdesk.rs", impl_symbols: "list_messages", donor: "chatwoot" },
  { key: "agent-knowledge.manage_conversation_draft", test_file: "crates/chronica-api/tests/helpdesk_drafts_route.rs", test_symbol: "draft_lifecycle_per_agent_and_scrub", impl_file: "crates/chronica-api/src/routes/helpdesk_drafts.rs", impl_symbols: "save_draft, get_draft, clear_draft", donor: "chatwoot" },
  { key: "agent-knowledge.mark_conversation_read_state", test_file: "crates/chronica-api/tests/helpdesk_read_state_route.rs", test_symbol: "mark_read_is_per_agent", impl_file: "crates/chronica-api/src/routes/helpdesk_read_state.rs", impl_symbols: "mark_read", donor: "chatwoot" },
  { key: "agent-knowledge.get_conversation_unread_counts", test_file: "crates/chronica-api/tests/helpdesk_read_state_route.rs", test_symbol: "mark_read_is_per_agent", impl_file: "crates/chronica-api/src/routes/helpdesk_read_state.rs", impl_symbols: "unread_counts", donor: "chatwoot" },
  { key: "agent-knowledge.add_conversation_labels", test_file: "crates/chronica-api/tests/helpdesk_label_route.rs", test_symbol: "thread_tagging_lifecycle", impl_file: "crates/chronica-api/src/routes/helpdesk_labels.rs", impl_symbols: "tag_thread, untag_thread", donor: "chatwoot" },
  { key: "agent-knowledge.get_contact_conversations", test_file: "crates/chronica-api/tests/helpdesk_contact_conversations_route.rs", test_symbol: "lists_only_the_contacts_linked_threads", impl_file: "crates/chronica-api/src/routes/helpdesk_contact_conversations.rs", impl_symbols: "list_contact_conversations", donor: "chatwoot" },
  { key: "agent-knowledge.update_conversation", test_file: "crates/chronica-api/tests/helpdesk_route.rs", test_symbol: "helpdesk_status_workflow_and_filter", impl_file: "crates/chronica-api/src/routes/helpdesk.rs", impl_symbols: "set_status, set_priority, assign", donor: "chatwoot" },
  { key: "agent-knowledge.bulk_conversation_action", test_file: "crates/chronica-api/tests/helpdesk_route.rs", test_symbol: "helpdesk_bulk_actions", impl_file: "crates/chronica-api/src/routes/helpdesk.rs", impl_symbols: "bulk_actions", donor: "chatwoot" },
  { key: "agent-knowledge.manage_conversation_participants", test_file: "crates/chronica-api/tests/helpdesk_participants_route.rs", test_symbol: "participant_lifecycle", impl_file: "crates/chronica-api/src/routes/helpdesk_participants.rs", impl_symbols: "add_participant, remove_participant, list_participants", donor: "chatwoot" },
  { key: "crm-support.manage_canned_response", test_file: "crates/chronica-api/tests/helpdesk_canned_route.rs", test_symbol: "canned_crud_happy_path", impl_file: "crates/chronica-api/src/routes/helpdesk_canned.rs", impl_symbols: "list_canned, create_canned, get_canned, update_canned, delete_canned", donor: "chatwoot" },
  { key: "crm-support.manage_label", test_file: "crates/chronica-api/tests/helpdesk_label_route.rs", test_symbol: "label_crud_happy_path", impl_file: "crates/chronica-api/src/routes/helpdesk_labels.rs", impl_symbols: "list_labels, create_label, update_label, delete_label, list_thread_labels, tag_thread, untag_thread", donor: "chatwoot" },
  { key: "crm-support.manage_custom_filter", test_file: "crates/chronica-api/tests/helpdesk_custom_filters_route.rs", test_symbol: "custom_filter_crud_happy_path", impl_file: "crates/chronica-api/src/routes/helpdesk_custom_filters.rs", impl_symbols: "create_filter, list_filters, update_filter, delete_filter", donor: "chatwoot" },
  { key: "crm-support.manage_contact", test_file: "crates/chronica-api/tests/helpdesk_contacts_route.rs", test_symbol: "contact_crud_happy_path", impl_file: "crates/chronica-api/src/routes/helpdesk_contacts.rs", impl_symbols: "list_contacts, create_contact, get_contact, update_contact, set_blocked, delete_contact", donor: "chatwoot" },
  { key: "crm-support.manage_contact_attributes", test_file: "crates/chronica-api/tests/helpdesk_custom_attribute_values_route.rs", test_symbol: "contact_attribute_values_set_get_merge", impl_file: "crates/chronica-api/src/routes/helpdesk_custom_attributes.rs", impl_symbols: "set_contact_attributes, get_contact_attributes", donor: "chatwoot" },
  { key: "crm-support.manage_contact_note", test_file: "crates/chronica-api/tests/helpdesk_contact_notes_route.rs", test_symbol: "contact_note_lifecycle_and_redaction", impl_file: "crates/chronica-api/src/routes/helpdesk_contact_notes.rs", impl_symbols: "add_note, list_notes, delete_note", donor: "chatwoot" },
  { key: "crm-support.manage_contact_inbox_link", test_file: "crates/chronica-api/tests/helpdesk_contacts_route.rs", test_symbol: "thread_contact_link_and_unlink", impl_file: "crates/chronica-api/src/routes/helpdesk.rs", impl_symbols: "set_contact", donor: "chatwoot" },
  { key: "crm-support.manage_team", test_file: "crates/chronica-api/tests/helpdesk_teams_route.rs", test_symbol: "team_crud_happy_path", impl_file: "crates/chronica-api/src/routes/helpdesk_teams.rs", impl_symbols: "create_team, list_teams, update_team, delete_team", donor: "chatwoot" },
  { key: "crm-support.manage_team_members", test_file: "crates/chronica-api/tests/helpdesk_teams_route.rs", test_symbol: "team_members_lifecycle", impl_file: "crates/chronica-api/src/routes/helpdesk_teams.rs", impl_symbols: "add_member, remove_member, list_members", donor: "chatwoot" },
  { key: "crm-support.manage_csat_survey", test_file: "crates/chronica-api/tests/helpdesk_csat_route.rs", test_symbol: "csat_record_get_update_and_summary", impl_file: "crates/chronica-api/src/routes/helpdesk_csat.rs", impl_symbols: "record_csat, get_csat, csat_summary", donor: "chatwoot" },
  { key: "crm-support.manage_message", test_file: "crates/chronica-api/tests/helpdesk_route.rs", test_symbol: "helpdesk_reply_author_from_principal_never_contact_or_system", impl_file: "crates/chronica-api/src/routes/helpdesk.rs", impl_symbols: "post_message, list_messages", donor: "chatwoot" },
  { key: "crm-support.manage_custom_attribute_definition", test_file: "crates/chronica-api/tests/helpdesk_custom_attributes_route.rs", test_symbol: "custom_attribute_crud_happy_path", impl_file: "crates/chronica-api/src/routes/helpdesk_custom_attributes.rs", impl_symbols: "list_defs, create_def, get_def, update_def, delete_def", donor: "chatwoot" },
  { key: "crm-support.manage_chat_comments", test_file: "crates/chronica-api/tests/helpdesk_comments_route.rs", test_symbol: "comment_lifecycle_and_scrub", impl_file: "crates/chronica-api/src/routes/helpdesk_comments.rs", impl_symbols: "add_comment, list_comments, delete_comment", donor: "chatwoot" },
  { key: "crm-support.manage_notifications", test_file: "crates/chronica-api/tests/helpdesk_notifications_route.rs", test_symbol: "notifications_list_count_mark_recipient_scoped", impl_file: "crates/chronica-api/src/routes/helpdesk_notifications.rs", impl_symbols: "list_notifications, unread_count, mark_read, mark_all_read", donor: "chatwoot" },
  { key: "workflow-runtime.create_automation_rule", test_file: "crates/chronica-api/tests/helpdesk_automation_route.rs", test_symbol: "automation_crud_happy_path", impl_file: "crates/chronica-api/src/routes/helpdesk_automations.rs", impl_symbols: "create_automation", donor: "chatwoot" },
  { key: "workflow-runtime.read_automation_rule", test_file: "crates/chronica-api/tests/helpdesk_automation_route.rs", test_symbol: "automation_crud_happy_path", impl_file: "crates/chronica-api/src/routes/helpdesk_automations.rs", impl_symbols: "get_automation, list_automations", donor: "chatwoot" },
  { key: "workflow-runtime.update_automation_rule", test_file: "crates/chronica-api/tests/helpdesk_automation_route.rs", test_symbol: "automation_crud_happy_path", impl_file: "crates/chronica-api/src/routes/helpdesk_automations.rs", impl_symbols: "update_automation", donor: "chatwoot" },
  { key: "workflow-runtime.delete_automation_rule", test_file: "crates/chronica-api/tests/helpdesk_automation_route.rs", test_symbol: "automation_crud_happy_path", impl_file: "crates/chronica-api/src/routes/helpdesk_automations.rs", impl_symbols: "delete_automation", donor: "chatwoot" },
  { key: "obs.analytics.get_csat_metrics", test_file: "crates/chronica-api/tests/helpdesk_csat_route.rs", test_symbol: "csat_record_get_update_and_summary", impl_file: "crates/chronica-api/src/routes/helpdesk_csat.rs", impl_symbols: "csat_summary", donor: "chatwoot" },
]

// ── (B) downgrades: cap has NO real production equivalent — delete the dangling evidence + drop verified. ──
const DOWNGRADES = [
  { key: "agent-knowledge.create_agent_bot", reason: "No agent-bot route or surface present. The de-fork deleted inbox/bots.rs and nothing reimplemented it; rg agent_bot/bot_token over crates/chronica-helpdesk + crates/chronica-api/src/routes returns zero hits. Bots involve credentials/secrets. No production equivalent -> delete evidence + drop verified." },
  { key: "agent-knowledge.show_agent_bot", reason: "No agent-bot surface present (bots.rs deleted, not reimplemented). rg returns zero hits across chronica-helpdesk + routes. No production equivalent -> downgrade." },
  { key: "agent-knowledge.update_agent_bot", reason: "No agent-bot surface present (bots.rs deleted, not reimplemented). No production equivalent -> downgrade." },
  { key: "agent-knowledge.delete_agent_bot", reason: "No agent-bot surface present (bots.rs deleted, not reimplemented). No production equivalent -> downgrade." },
  { key: "agent-knowledge.reset_agent_bot_credentials", reason: "No agent-bot credential/token surface present (verify_bot_token absent everywhere; bots.rs deleted, not reimplemented). Secret reset has no production equivalent -> downgrade." },
  { key: "agent-knowledge.set_inbox_agent_bot", reason: "No agent-bot surface AND no Inbox-registry surface present to attach a bot to (both bots.rs and inboxes.rs deleted, not reimplemented). No production equivalent -> downgrade." },
  { key: "agent-knowledge.upload_conversation_attachment", reason: "No attachment upload implementation present; requires object-storage which is not wired. rg attachment over crates/chronica-helpdesk hits only a doc-comment in domain/message.rs ('drafts/attachments/status') describing future scope — no AttachmentRef type, no upload handler, no route. messages.rs deleted, attachments never reimplemented -> downgrade." },
  { key: "agent-knowledge.get_conversation_attachments", reason: "No attachment listing implementation present; requires object-storage which is not wired. Only a doc-comment mention in message.rs; no handler/route/store. -> downgrade." },
  { key: "agent-knowledge.delete_conversation", reason: "No conversation-delete handler present. helpdesk.rs threads expose GET plus status/priority/assign/contact/snooze sub-routes only; rg fn delete_thread / DELETE .../threads returns nothing. Threads are never hard-deleted in the present surface. No production equivalent -> downgrade." },
  { key: "crm-support.search_filter_contacts", reason: "No real contact search/filter endpoint present. Verified by reading helpdesk_contacts.rs: list_contacts takes only company + MAX_LIST and calls contact_read::list_contacts with no q/search/filter param; contact_read.rs has no ILIKE/query path. The candidate test contact_validation_matrix exercises create/validation, not search. The Chatwoot advanced-filter contact capability is not implemented -> downgrade until a real search query+test exists." },
  { key: "crm-support.bulk_contact_action", reason: "No bulk-contacts endpoint or handler. helpdesk_contacts.rs registers only list/create + get/patch/delete(:contact_id) + /blocked; helpdesk.rs bulk_actions operates on THREADS only, not contacts. rg bulk_contact over crates returns nothing. No production equivalent -> downgrade." },
  { key: "crm-support.import_export_contacts", reason: "No import/export endpoint or handler anywhere in the contacts route; rg import_contact/export_contact/csv over crates/chronica-helpdesk + routes returns nothing. contacts.rs deleted, import/export never reimplemented -> downgrade." },
  { key: "crm-support.manage_inbox", reason: "No Inbox-registry/channel-inbox CRUD route present. The de-fork dropped the Chatwoot multi-inbox/channel model; rg helpdesk/inbox(es)/InboxRegistry over crates/chronica-api/src/routes returns nothing. (store/read_model.rs InboxRow is the agent inbox-LIST read-model, a different concept.) -> downgrade." },
  { key: "crm-support.manage_inbox_members", reason: "No Inbox-registry surface present (see manage_inbox) and no inbox_member implementation. inboxes.rs deleted, not reimplemented -> downgrade." },
  { key: "crm-support.manage_campaign", reason: "No campaign route or surface present anywhere. rg campaign over crates/chronica-helpdesk + routes returns nothing. campaigns.rs deleted, never reimplemented -> downgrade." },
  { key: "policy.inbox.reset_api_secret", reason: "No inbox API-secret surface present; depends on the absent Inbox registry. rg api_secret/reset_api_secret/verify_api_secret over crates returns nothing -> downgrade." },
  { key: "policy.assignment.manage_assignment_policy", reason: "No assignment-policy route or surface present; depends on the absent Inbox registry. rg assignment_policy/AssignmentPolicy returns nothing. (The present assign handler does manual assignment only, no auto-assignment policy.) -> downgrade." },
  { key: "infra.assignment.list_policies", reason: "No assignment-policy listing present (see manage_assignment_policy). rg list_assignment_policies returns nothing -> downgrade." },
  { key: "workflow-runtime.clone_automation_rule", reason: "No clone-automation-rule endpoint. helpdesk_automations.rs implements list/create/get/update/delete only; rg clone_automation/fn clone over the route + domain/automation.rs returns nothing. The clone operation is not implemented (the other 4 automation CRUD caps ARE present and re-pointed) -> downgrade." },
]

const BLOCKER = 'Chatwoot inbox pure-logic removed in the chronica-helpdesk de-fork (deleted crates/chronica-erp/src/inbox/*); not yet reimplemented as an event-driven chronica-helpdesk module. Escalated (object-storage / credentials / inbox-registry design).'

const fileCache = new Map()
const read = (p) => { if (fileCache.has(p)) return fileCache.get(p); const s = existsSync(p) ? readFileSync(p, 'utf8') : null; fileCache.set(p, s); return s }
const hasTestFn = (src, sym) => new RegExp(`\\bfn\\s+${sym}\\b`).test(src)
const hasSym = (src, sym) => new RegExp(`\\b${sym.trim()}\\b`).test(src)

const db = new Database(CAPABILITIES_DB)
const allKeys = [...REPOINTS.map(r => r.key), ...DOWNGRADES.map(d => d.key)]
const placeholders = allKeys.map(() => '?').join(',')
const existingKeys = new Set(placeholders
  ? db.prepare(`SELECT key FROM canonical_capability WHERE key IN (${placeholders})`).all(...allKeys).map(r => r.key)
  : [])
const activeRepoints = REPOINTS.filter(r => existingKeys.has(r.key))
const activeDowngrades = DOWNGRADES.filter(d => existingKeys.has(d.key))
const skippedMissing = allKeys.filter(k => !existingKeys.has(k))

// ── GUARD: validate EVERY existing re-point against real source before any write (all-or-nothing). ──
const failures = []
for (const r of activeRepoints) {
  const tsrc = read(r.test_file), isrc = read(r.impl_file)
  if (!tsrc) { failures.push(`${r.key}: test_file missing ${r.test_file}`); continue }
  if (!hasTestFn(tsrc, r.test_symbol)) failures.push(`${r.key}: test fn \`${r.test_symbol}\` absent in ${r.test_file}`)
  if (!isrc) { failures.push(`${r.key}: impl_file missing ${r.impl_file}`); continue }
  for (const s of r.impl_symbols.split(',').map(x => x.trim()).filter(Boolean)) {
    if (!hasSym(isrc, s)) failures.push(`${r.key}: impl symbol \`${s}\` absent in ${r.impl_file}`)
  }
}
if (failures.length) {
  console.error(`REFUSED: ${failures.length} re-point guard failure(s) — wrote NOTHING:`)
  for (const f of failures) console.error(`  ✗ ${f}`)
  process.exit(1)
}
const nowIso = new Date().toISOString()
const by = 'loop:reconcile-helpdesk-evidence'

const repointCap = db.prepare("UPDATE canonical_capability SET moves_money=0, side_effect_class='internal_write', status='verified', acceptance_test=?, financial_control_test=NULL WHERE key=?")
const repointOverride = db.prepare(`INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,financial_control_test,moves_money,set_at,set_by)
  VALUES (?, 'verified', ?, NULL, 0, ?, ?)
  ON CONFLICT(canonical_key) DO UPDATE SET status='verified', acceptance_test=excluded.acceptance_test, financial_control_test=NULL, moves_money=0, set_at=excluded.set_at, set_by=excluded.set_by`)
const repointEvidence = db.prepare(`INSERT INTO impl_evidence (canonical_key,impl_file,impl_symbols,test_file,test_symbol,donor_source,verified_at,verified_by,notes)
  VALUES (@key,@impl,@symbols,@testFile,@test,@donor,@now,@by,@notes)
  ON CONFLICT(canonical_key) DO UPDATE SET impl_file=@impl, impl_symbols=@symbols, test_file=@testFile, test_symbol=@test, donor_source=@donor, verified_at=@now, verified_by=@by, notes=@notes`)

const dropEvidence = db.prepare('DELETE FROM impl_evidence WHERE canonical_key=?')
// downgrade: drop verified status to 'unimplemented' + blocker. moves_money/requires_approval are NOT touched.
const downgradeCap = db.prepare("UPDATE canonical_capability SET status='unimplemented', blocker=? WHERE key=?")
const downgradeOverride = db.prepare(`INSERT INTO canonical_status_override (canonical_key,status,blocker,set_at,set_by)
  VALUES (?, 'unimplemented', ?, ?, ?)
  ON CONFLICT(canonical_key) DO UPDATE SET status='unimplemented', blocker=excluded.blocker, set_at=excluded.set_at, set_by=excluded.set_by`)

const tx = db.transaction(() => {
  for (const r of activeRepoints) {
    repointCap.run(r.test_symbol, r.key)
    repointOverride.run(r.key, r.test_symbol, nowIso, by)
    repointEvidence.run({ key: r.key, impl: r.impl_file, symbols: r.impl_symbols, testFile: r.test_file, test: r.test_symbol, donor: r.donor, now: nowIso, by, notes: 'chronica-helpdesk de-fork re-point (served-by route test)' })
  }
  for (const d of activeDowngrades) {
    dropEvidence.run(d.key)
    downgradeCap.run(BLOCKER, d.key)
    downgradeOverride.run(d.key, BLOCKER, nowIso, by)
  }
})
tx()

// ── self-check: none of the touched keys may dangle, and downgrades must have no evidence row. ──
let bad = 0
for (const r of activeRepoints) {
  const e = db.prepare('SELECT impl_file,impl_symbols,test_file,test_symbol FROM impl_evidence WHERE canonical_key=?').get(r.key)
  const tsrc = read(e.test_file), isrc = read(e.impl_file)
  if (!tsrc || !hasTestFn(tsrc, e.test_symbol) || !isrc || (e.impl_symbols || '').split(',').some(s => s.trim() && !hasSym(isrc, s))) { console.error(`  ✗ post-check dangle ${r.key}`); bad++ }
}
for (const d of activeDowngrades) {
  if (db.prepare('SELECT 1 FROM impl_evidence WHERE canonical_key=?').get(d.key)) { console.error(`  ✗ downgrade still has evidence ${d.key}`); bad++ }
  const st = db.prepare('SELECT status FROM canonical_capability WHERE key=?').get(d.key).status
  if (st === 'verified') { console.error(`  ✗ downgrade still verified ${d.key}`); bad++ }
}
db.close()
console.log(`reconcile-helpdesk-evidence: re-pointed ${activeRepoints.length} caps to live chronica-helpdesk code, downgraded ${activeDowngrades.length} unbuilt caps to 'unimplemented', skipped ${skippedMissing.length} absent legacy cap(s). moves_money untouched (all helpdesk caps stay 0).`)
if (bad) { console.error(`  ✗ ${bad} post-write self-check failure(s)`); process.exit(1) }
console.log('  ✓ self-check clean — run verify-impl-evidence to confirm 0 dangling.')
