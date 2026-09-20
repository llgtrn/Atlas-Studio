#!/usr/bin/env node
// backfill-missing-target-crates.mjs — one-shot backfill of the 144 canonical capabilities that
// had NULL/empty target_crate (architecture_target_gap kind `missing_target_metadata`).
//
// WHY: every canonical capability must land in exactly one honest architecture coverage state
// (tools/architecture/architecture-lib.mjs). A NULL target_crate is an actionable metadata bug —
// the capability cannot be linked to its owning live crate nor honestly deferred.
//
// HOW each home was assigned (2026-06-10 gap-closure round, NO architecture theater):
//   1. status=verified caps: home = the crate of the REAL implementing module per impl_evidence
//      (policy.approval.manage_quote_lifecycle / run_antispam_preflight →
//       crates/chronica-observability/src/approval/{quote_lifecycle,antispam}.rs).
//   2. status=unimplemented caps: home = the code-anchored sibling convention —
//      the live crate where caps of the same domain/prefix already live
//      (e.g. obs.otel.* siblings incl. 3 verified → chronica-observability; osint.* → chronica-osint;
//       billing webhooks → chronica-security webhook_events.rs precedent; router.*/llm.* → the
//       chronica-runtime route_model/load_balancing home; ssrf → chronica-security src/ssrf/guard.rs).
//   3. Genuinely cross-cutting caps were placed by the documented LOGICAL_CRATE_HOME conventions
//      (documents/search/storage → chronica-memory; channels/notifications → chronica-integrations).
//
// SAFETY: updates ONLY target_crate (and target_module for the two verified caps with known
// modules). NEVER touches status, moves_money, requires_approval, or any approval flag.
// Idempotent: only fills rows whose target_crate is still NULL/empty — it never clobbers a
// later correction. Safe to re-run after a census rebuild that re-creates rows with NULL targets.
import Database from 'better-sqlite3'
import { CAPABILITIES_DB } from '../_paths.mjs'

// key -> live workspace target crate (39-crate workspace as of 2026-06-10)
const TARGETS = {
  // ── agent-knowledge: ACP/agent-runtime verbs → chronica-runtime; captain (chatwoot AI
  //    assistant) features → chronica-tools (support_assistant.rs home); trajectory →
  //    chronica-traces (ai_assistant_conversations.rs precedent); embeddings → chronica-memory
  'agent-knowledge.bind_persistent_agent_session': 'chronica-runtime',
  'agent-knowledge.gate_acp_dispatch_policy': 'chronica-runtime',
  'agent-knowledge.register_acp_runtime_backend': 'chronica-runtime',
  'agent-knowledge.resolve_managed_agent_binary': 'chronica-runtime',
  'agent-knowledge.record_agent_trajectory': 'chronica-traces',
  'agent-knowledge.serve_embeddings_api': 'chronica-memory',
  'agent-knowledge.manage_captain_model_preferences': 'chronica-tools',
  'agent-knowledge.refine_captain_task_follow_up': 'chronica-tools',
  'agent-knowledge.setup_agent_computer_use': 'chronica-tools',
  'agent-knowledge.track_help_center_generation_progress': 'chronica-tools',
  // ── browser-internet-hand: crawling → chronica-internet-hand (the web-hand crate)
  'crawl.manage_keywords': 'chronica-internet-hand',
  'crawl.persist_crawled_data': 'chronica-internet-hand',
  // ── crm-support: CRM/inbox entities → chronica-erp (chronica-crm/inbox logical homes);
  //    external channels (mail/slack/whatsapp/push/translate/bots) → chronica-integrations
  //    (email_channel.rs / emailing_domain.rs code-verified siblings); attachment → chronica-artifacts
  'crm-support.authenticate_mail_plugin': 'chronica-integrations',
  'crm-support.auto_link_linear_issues': 'chronica-integrations',
  'crm-support.detect_conversation_language': 'chronica-integrations',
  'crm-support.email_livechat_transcript': 'chronica-integrations',
  'crm-support.email_portal_cname_instructions': 'chronica-integrations',
  'crm-support.fetch_link_preview': 'chronica-integrations',
  'crm-support.process_edited_channel_messages': 'chronica-integrations',
  'crm-support.provision_whatsapp_cloud_channel': 'chronica-integrations',
  'crm-support.run_bot_processor_pipeline': 'chronica-integrations',
  'crm-support.run_realtime_voice_video_call': 'chronica-integrations',
  'crm-support.send_web_push_notification': 'chronica-integrations',
  'crm-support.unfurl_conversation_links_in_slack': 'chronica-integrations',
  'crm-support.autocomplete_partner_via_iap': 'chronica-erp',
  'crm-support.bootstrap_widget_visitor_session': 'chronica-erp',
  'crm-support.create_ticket': 'chronica-erp',
  'crm-support.manage_livechat_session_metadata': 'chronica-erp',
  'crm-support.query_ticket_history': 'chronica-erp',
  'crm-support.serve_messaging_store_data': 'chronica-erp',
  'crm-support.upload_account_attachment_safefetch': 'chronica-artifacts',
  // ── erp-finance: accounting verbs → chronica-erp (accounting/ + market_data.rs siblings)
  'erp.accounting.post_cogs_entry': 'chronica-erp',
  'erp.fetch_fx_exchange_rate': 'chronica-erp',
  // ── infra-execution
  'infra.avatar.get_images': 'chronica-policy', // user-profile asset (governance/user_profile.rs home)
  'infra.cli.settings_builder': 'chronica-cli',
  'infra.config.cloud_sync': 'chronica-integrations', // cloud-client logical home
  'infra.export_prometheus_metrics': 'chronica-observability',
  'infra.manage_webhook_subscriptions': 'chronica-integrations', // outbound webhook connectors
  'infra.migrate_assistant_secrets': 'chronica-security', // auth/secret store owner
  // ── media-generation: document-IR/report pipeline → chronica-memory (doc_format.rs +
  //    documents/pdf-export/search logical homes; NOT BI dashboards, NOT DSP media)
  'media.bind_document_ir': 'chronica-memory',
  'media.validate_document_ir': 'chronica-memory',
  'media.render_report_html': 'chronica-memory',
  'media.render_report_pdf': 'chronica-memory',
  'media.cluster_search_results': 'chronica-memory',
  // ── observability-analytics: otel collector pipeline → chronica-observability (obs.otel.*
  //    siblings incl. verified batch/classify/limit caps); ingest/presence → chronica-events;
  //    statistical/anomaly models → chronica-analytics; ML inference → chronica-runtime;
  //    vectors → chronica-memory
  'obs.agent.monitor_deliberation': 'chronica-observability',
  'obs.cluster_correlate_events': 'chronica-observability',
  'obs.otel.build_pipeline_topology': 'chronica-observability',
  'obs.otel.connect_pipelines': 'chronica-observability',
  'obs.otel.export_telemetry': 'chronica-observability',
  'obs.otel.filter_telemetry': 'chronica-observability',
  'obs.otel.manage_internal_telemetry_format': 'chronica-observability',
  'obs.otel.monitor_collector_self': 'chronica-observability',
  'obs.otel.process_telemetry_pipeline': 'chronica-observability',
  'obs.otel.receive_telemetry': 'chronica-observability',
  'obs.broadcast_realtime_presence': 'chronica-events',
  'obs.ingest_widget_visitor_events': 'chronica-events',
  'obs.detect_temporal_anomalies': 'chronica-analytics',
  'obs.run_statistical_model': 'chronica-analytics',
  'obs.run_clientside_ml_inference': 'chronica-runtime',
  'obs.store_search_vectors_client': 'chronica-memory',
  // ── osint: intel/recon/data-service caps → chronica-osint (151 siblings); outbound email
  //    campaigns + MCP protocol serving → chronica-integrations (channel/MCP logical homes)
  'osint.aggregate_price_indices': 'chronica-osint',
  'osint.chat_analyst_agent': 'chronica-osint',
  'osint.classify_news_threat': 'chronica-osint',
  'osint.compute_chokepoint_exposure': 'chronica-osint',
  'osint.compute_economic_stress_index': 'chronica-osint',
  'osint.compute_resilience_ranking': 'chronica-osint',
  'osint.compute_shipping_route_intel': 'chronica-osint',
  'osint.compute_supply_shock_scenario': 'chronica-osint',
  'osint.crawl_social_platforms_by_keyword': 'chronica-osint',
  'osint.fetch_economic_indicator': 'chronica-osint',
  'osint.fetch_webcam_imagery': 'chronica-osint',
  'osint.lookup_sanction_entity': 'chronica-osint',
  'osint.normalize_match_products': 'chronica-osint',
  'osint.publish_price_datasets': 'chronica-osint',
  'osint.query_country_risk': 'chronica-osint',
  'osint.scrape_retailer_prices': 'chronica-osint',
  'osint.search_gdelt_documents': 'chronica-osint',
  'osint.search_google_flights': 'chronica-osint',
  'osint.serve_consumer_price_index': 'chronica-osint',
  'osint.serve_energy_commodity_data': 'chronica-osint',
  'osint.serve_energy_infrastructure': 'chronica-osint',
  'osint.serve_forecast_scenarios': 'chronica-osint',
  'osint.serve_macro_indicators': 'chronica-osint',
  'osint.serve_military_intel': 'chronica-osint',
  'osint.serve_research_signals': 'chronica-osint',
  'osint.serve_social_signals': 'chronica-osint',
  'osint.serve_supply_chain_data': 'chronica-osint',
  'osint.serve_trade_flows': 'chronica-osint',
  'osint.track_aircraft_flights': 'chronica-osint',
  'osint.send_broadcast_email': 'chronica-integrations',
  'osint.serve_mcp_server': 'chronica-integrations',
  // ── other
  'billing.process_payment_webhook': 'chronica-security', // webhook_events.rs billing-webhook precedent
  'browser.serve_acp_ide_bridge': 'chronica-runtime', // ACP backends register in runtime
  'channel.manage': 'chronica-integrations',
  'chart.validate_repair': 'chronica-analytics',
  'config.system.access': 'chronica-policy',
  'infra.boot_nodes': 'chronica-kubernetes',
  'infra.install_token': 'chronica-kubernetes',
  'infra.media.extract_video_metadata': 'chronica-media',
  'llm.count_tokens': 'chronica-runtime',
  'llm.custom_model_define': 'chronica-runtime',
  'llm.list_models': 'chronica-runtime',
  'llm.model_disable_per_provider': 'chronica-runtime',
  'news.collect_topics': 'chronica-osint',
  'nlp.sentiment_analyze': 'chronica-osint', // news/intel sentiment (analysis logical home)
  'noise.auth.account_fallback_roundrobin': 'chronica-runtime', // provider-account routing (load_balancing home)
  'opinion.query_db': 'chronica-simulation', // social-simulation opinion store
  'otel.component_lifecycle': 'chronica-observability',
  'otel.connector_pipeline': 'chronica-observability',
  'otel.consumer_errors': 'chronica-observability',
  'otel.protocol_config': 'chronica-observability',
  'otel.receive_profiles': 'chronica-observability',
  'otel.receiver_factory': 'chronica-observability',
  'otel.report_status': 'chronica-observability',
  'policy.secrets.block_private_address_ssrf': 'chronica-security', // src/ssrf/guard.rs is the SSRF home
  'report.layout_design': 'chronica-memory',
  'report.regenerate_artifacts': 'chronica-memory',
  'router.api_compat': 'chronica-runtime',
  'router.bypass_warmup': 'chronica-runtime',
  'router.caveman_compression': 'chronica-runtime',
  'router.project_id_resolve': 'chronica-runtime',
  'router.provider_adapter': 'chronica-runtime',
  'router.streaming_sse': 'chronica-runtime',
  'router.thinking_routing': 'chronica-runtime',
  'search.multimodal': 'chronica-osint', // live web/news search → recon home
  'search.news': 'chronica-osint',
  'search.optimize_keywords': 'chronica-memory', // database search keywords → search home
  'state.update_global': 'chronica-workflows', // graph/state engine home
  'storage.data_buckets': 'chronica-memory',
  'taxonomy.define_categories': 'chronica-memory',
  // ── policy-approval-audit: the two VERIFIED caps live in chronica-observability/src/approval/
  //    per impl_evidence; severance is the same mastodon approval-suite donor pocket
  'policy.approval.manage_quote_lifecycle': 'chronica-observability',
  'policy.approval.run_antispam_preflight': 'chronica-observability',
  'policy.audit.record_relationship_severance': 'chronica-observability',
  'policy.authorize_support_resource_actions': 'chronica-policy', // RBAC/ACL rule home
  'policy.operate_assistant_console': 'chronica-tools', // assistant console (support_assistant home)
  'policy.verify_widget_contact_hmac': 'chronica-security', // webhook_verify HMAC primitive home
  // ── trading-markets → chronica-strategy (trading/markets/portfolio logical homes)
  'trading-markets.fetch_economic_indicator': 'chronica-strategy',
  'trading-markets.persist_market_store': 'chronica-strategy',
  // ── unclustered
  'unclustered.manage_api_keys_1291': 'chronica-security', // api_key home (chronica-auth → security)
  'unclustered.provide_extension_functionality_to_compo_1103': 'chronica-tools', // extensions home
  // ── workflow-runtime: job/task queue → chronica-workflows (manage_task_queue siblings);
  //    telemetry signal pipelines → chronica-observability (otel pipeline home)
  'workflow-runtime.define_signal_pipelines': 'chronica-observability',
  'workflow-runtime.handle_report_task_queue': 'chronica-workflows',
  'workflow-runtime.query_job_history': 'chronica-workflows',
  'workflow-runtime.run_job_actions': 'chronica-workflows',
}

// target_module for caps whose REAL implementing module is known from impl_evidence.
const MODULES = {
  'policy.approval.manage_quote_lifecycle': 'approval/quote_lifecycle',
  'policy.approval.run_antispam_preflight': 'approval/antispam',
}

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')

const updCrate = db.prepare(`UPDATE canonical_capability SET target_crate = ?
  WHERE key = ? AND (target_crate IS NULL OR TRIM(target_crate) = '')`)
const updModule = db.prepare(`UPDATE canonical_capability SET target_module = ?
  WHERE key = ? AND (target_module IS NULL OR TRIM(target_module) = '')`)
const exists = db.prepare('SELECT target_crate FROM canonical_capability WHERE key = ?')

let filled = 0
let alreadySet = 0
const missingKeys = []
const tx = db.transaction(() => {
  for (const [key, crate] of Object.entries(TARGETS)) {
    const row = exists.get(key)
    if (!row) { missingKeys.push(key); continue }
    const changed = updCrate.run(crate, key).changes
    if (changed) filled += 1
    else alreadySet += 1
    if (MODULES[key]) updModule.run(MODULES[key], key)
  }
  if (filled > 0) {
    db.prepare(`INSERT INTO agent_note (author, kind, status, body, created_at)
      VALUES ('claude', 'note', 'resolved', ?, ?)`).run(
      `architecture-gap closure: backfilled target_crate for ${filled} canonical capabilities that had NULL/empty target_crate (gap kind missing_target_metadata). Homes assigned from impl_evidence (2 verified caps -> chronica-observability approval/), code-anchored sibling conventions, and documented LOGICAL_CRATE_HOME conventions. Script: tools/capabilities/backfill-missing-target-crates.mjs. No status/moves_money/approval flags touched.`,
      new Date().toISOString(),
    )
  }
})
tx()

const remaining = db.prepare(`SELECT count(*) n FROM canonical_capability
  WHERE target_crate IS NULL OR TRIM(target_crate) = ''`).get().n

console.log(`backfill-missing-target-crates: ${Object.keys(TARGETS).length} keys mapped`)
console.log(`  filled=${filled} already_set(skipped)=${alreadySet} keys_not_found=${missingKeys.length}`)
for (const k of missingKeys) console.log(`    NOT FOUND: ${k}`)
console.log(`  canonical rows still missing target_crate: ${remaining}`)
if (remaining > 0) {
  const left = db.prepare(`SELECT key FROM canonical_capability
    WHERE target_crate IS NULL OR TRIM(target_crate) = '' LIMIT 20`).all()
  for (const r of left) console.log(`    still missing: ${r.key}`)
}
console.log('next: pnpm arch:build && pnpm arch:verify && pnpm docs:gen && pnpm docs:verify')
