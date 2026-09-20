#!/usr/bin/env node
// Normalize n8n/Dify/Refly donor targets onto real Chronica-native crates.
//
// This is intentionally not a census rebuild. It corrects DB-facing execution
// targets so implementation agents stop chasing donor-era or nonexistent crate
// names such as chronica-rag, chronica-workflow-engine, and openfang-runtime.

import Database from 'better-sqlite3'
import { CAPABILITIES_DB } from '../_paths.mjs'

const DONORS = ['n8n-master', 'dify', 'refly-main']
const now = new Date().toISOString()

const db = new Database(CAPABILITIES_DB)
db.pragma('journal_mode = WAL')

function normalizeTarget(row) {
  const key = String(row.key || '')
  const domain = String(row.domain || '')
  const crate = String(row.target_crate || '').trim()
  const name = String(row.canonical_name || row.canonical_name_source || '').toLowerCase()
  const text = `${key} ${domain} ${crate} ${name}`.toLowerCase()

  if (
    key === 'workflow-runtime.manage_human_input' ||
    key === 'workflow-runtime.manage_human_approval_queue' ||
    key === 'workflow-runtime.ask_user_question' ||
    key === 'workflow-runtime.execute_form_submission' ||
    key === 'agent-knowledge.tool_ask_user' ||
    key === 'agent-knowledge.assemble_agent_context' ||
    key === 'agent-knowledge.execute_agent_loop' ||
    key === 'workflow-runtime.compute_canvas_graph_ops' ||
    key === 'workflow-runtime.manage_canvas' ||
    key === 'workflow-runtime.manage_apps' ||
    key === 'workflow-runtime.run_workflow_app' ||
    key === 'workflow-runtime.manage_sharing' ||
    key === 'app.create' ||
    key === 'app.export' ||
    key === 'app.import' ||
    key === 'workflow-runtime.provide_workflow_tools' ||
    key === 'workflow-runtime.invoke_tool' ||
    key === 'workflow-runtime.manage_tool_calls' ||
    key === 'workflow-runtime.construct_cron_expression' ||
    key === 'workflow-runtime.extract_variables_from_prompt' ||
    key === 'workflow-runtime.manage_remote_triggers' ||
    key === 'workflow-runtime.webhook_trigger' ||
    key === 'workflow-runtime.schedule_cron_job' ||
    key === 'workflow-runtime.manage_workflow_run_lifecycle' ||
    key === 'workflow-runtime.queue_and_prioritize_execution' ||
    key === 'workflow-runtime.recover_crashed_executions' ||
    key === 'workflow-runtime.register_lifecycle_hooks' ||
    key === 'workflow-runtime.manage_task_queue' ||
    key === 'workflow-runtime.manage_dlq_tasks' ||
    key === 'workflow-runtime.enforce_job_limits' ||
    key === 'workflow-runtime.poll_job' ||
    key === 'workflow-runtime.replay_dlq_messages' ||
    key === 'workflow-runtime.conditional_branching' ||
    key === 'workflow-runtime.loop_iteration' ||
    key === 'workflow-runtime.parallel_execution' ||
    key === 'infra.model_workflow_execution_state'
  ) {
    return 'chronica-workflows'
  }

  if (key === 'policy.approval.request_user_approval') {
    return 'chronica-workflows'
  }

  if (key === 'data.compress') {
    return 'chronica-workflows'
  }

  if (
    key === 'obs.infra.compress_request' ||
    key === 'obs.logs.store_log_chunks'
  ) {
    return 'chronica-observability'
  }

  if (
    key === 'workflow-runtime.track_activity_feed' ||
    key === 'comment.manage'
  ) {
    return 'chronica-company'
  }

  if (
    key === 'obs.trace_agent_runs' ||
    key === 'obs.track_llm_token_usage' ||
    key === 'erp.billing.calculate_cost' ||
    key === 'erp.credit.track_usage'
  ) {
    return 'chronica-observability'
  }

  if (key === 'agent-knowledge.persist_assistant_conversations') {
    return 'chronica-traces'
  }

  if (
    key === 'agent-knowledge.configure_llm_provider' ||
    key === 'agent-knowledge.provide_llm_runtime_providers' ||
    key === 'agent-knowledge.build_provider_clients' ||
    key === 'agent-knowledge.check_provider_health' ||
    key === 'agent-knowledge.validate_openai_credential'
  ) {
    return 'chronica-runtime'
  }

  if (
    key === 'agent-knowledge.enforce_tool_permissions' ||
    key === 'agent-knowledge.classify_assistant_action'
  ) {
    return 'chronica-runtime'
  }

  if (key === 'agent.invoke_tool_call') {
    return 'chronica-runtime'
  }

  if (key === 'agent.tool_calls_resolve') {
    return 'chronica-runtime'
  }

  if (key === 'agent-knowledge.run_tool_handler_pipeline') {
    return 'chronica-integrations'
  }

  if (key === 'agent-knowledge.manage_mcp_servers') {
    return 'chronica-integrations'
  }

  if (key === 'agent-knowledge.provide_integration_tools') {
    return 'chronica-integrations'
  }

  if (key === 'agent-knowledge.execute_ptc_tool') {
    return 'chronica-integrations'
  }

  if (key === 'agent-knowledge.manage_file_uploads') {
    return 'chronica-memory'
  }

  if (
    key === 'dataset.manage' ||
    key === 'agent-knowledge.embed_chunk_documents' ||
    key === 'agent-knowledge.build_semantic_index' ||
    key === 'agent-knowledge.serve_citation_docs'
  ) {
    return 'chronica-memory'
  }

  if (
    text.includes('human_input') ||
    text.includes('human input') ||
    text.includes('ask_user') ||
    text.includes('ask-user') ||
    text.includes('pause')
  ) {
    return 'chronica-workflows'
  }

  if (text.includes('webhook') || text.includes('cron') || text.includes('schedule')) {
    return text.includes('workflow') ? 'chronica-workflows' : 'chronica-scheduler'
  }

  if (
    domain === 'agent-knowledge' ||
    text.includes('rag') ||
    text.includes('semantic') ||
    text.includes('embedding') ||
    text.includes('vector') ||
    text.includes('fulltext') ||
    text.includes('knowledge') ||
    text.includes('search_library') ||
    text.includes('parse_documents') ||
    text.includes('ingest_documents')
  ) {
    if (
      text.includes('provider') ||
      text.includes('model') ||
      text.includes('route_model') ||
      text.includes('agent_loop') ||
      text.includes('sandbox') ||
      text.includes('tool_call')
    ) {
      return 'chronica-runtime'
    }
    if (
      text.includes('connector') ||
      text.includes('composio') ||
      text.includes('mcp') ||
      text.includes('integration') ||
      text.includes('tool')
    ) {
      return 'chronica-integrations'
    }
    return 'chronica-memory'
  }

  if (
    domain === 'workflow-runtime' ||
    text.includes('workflow') ||
    text.includes('canvas') ||
    text.includes('branch') ||
    text.includes('loop') ||
    text.includes('action_result') ||
    text.includes('app.create') ||
    text.includes('app.export') ||
    text.includes('app.import') ||
    text.includes('prompt')
  ) {
    return 'chronica-workflows'
  }

  if (
    domain === 'infra-execution' ||
    text.includes('execution') ||
    text.includes('queue') ||
    text.includes('worker') ||
    text.includes('runtime') ||
    text.includes('sandbox') ||
    text.includes('batch')
  ) {
    return 'chronica-runtime'
  }

  if (
    domain === 'erp-finance' ||
    text.includes('billing') ||
    text.includes('credit') ||
    text.includes('voucher') ||
    text.includes('subscription') ||
    text.includes('cost')
  ) {
    return 'chronica-observability'
  }

  if (
    domain === 'observability-analytics' ||
    text.includes('trace') ||
    text.includes('log') ||
    text.includes('analytics') ||
    text.includes('telemetry') ||
    text.includes('langfuse')
  ) {
    return 'chronica-observability'
  }

  if (
    domain === 'policy-approval-audit' ||
    text.includes('auth') ||
    text.includes('api_key') ||
    text.includes('secret') ||
    text.includes('credential') ||
    text.includes('permission') ||
    text.includes('retention')
  ) {
    return 'chronica-security'
  }

  if (
    text.includes('connector') ||
    text.includes('oauth') ||
    text.includes('http_request') ||
    text.includes('notion') ||
    text.includes('gmail') ||
    text.includes('github') ||
    text.includes('external')
  ) {
    return 'chronica-integrations'
  }

  if (
    text.includes('conversation') ||
    text.includes('message') ||
    text.includes('comment') ||
    text.includes('feedback') ||
    text.includes('workspace') ||
    text.includes('user.manage') ||
    text.includes('membership')
  ) {
    return 'chronica-erp'
  }

  if (text.includes('media') || text.includes('speech') || text.includes('audio')) {
    return 'chronica-media'
  }

  if (text.includes('template')) {
    return 'chronica-template-store'
  }

  if (crate.startsWith('chronica-') && KNOWN_CRATES.has(crate)) {
    return crate
  }

  return 'chronica-core'
}

const KNOWN_CRATES = new Set([
  'chronica-core',
  'chronica-policy',
  'chronica-approvals',
  'chronica-artifacts',
  'chronica-observability',
  'chronica-tools',
  'chronica-runtime',
  'chronica-scheduler',
  'chronica-workflows',
  'chronica-worker-supervisor',
  'chronica-company',
  'chronica-strategy',
  'chronica-osint',
  'chronica-internet-hand',
  'chronica-memory',
  'chronica-integrations',
  'chronica-simulation',
  'chronica-evidence-debate',
  'chronica-security',
  'chronica-world-model',
  'chronica-erp',
  'chronica-events',
  'chronica-analytics',
  'chronica-traces',
  'chronica-olap',
  'chronica-template',
  'chronica-template-store',
  'chronica-marketplaces',
  'chronica-commerce-signals',
  'chronica-commerce',
  'chronica-commerce-analytics',
  'chronica-replication',
  'chronica-routines',
  'chronica-api',
  'chronica-cli',
  'chronica-media',
  'chronica-kubernetes',
])

const sourceRows = db
  .prepare(
    `SELECT s.id, s.donor, c.key, s.canonical_name AS canonical_name_source, s.target_crate, s.target_module
       FROM source_capability s
       JOIN canonical_capability c ON c.id = s.canonical_id
      WHERE s.donor IN (${DONORS.map(() => '?').join(',')})`,
  )
  .all(...DONORS)

const canonicalRows = db
  .prepare(
    `SELECT DISTINCT c.id, c.key, c.canonical_name, c.domain, c.target_crate, c.target_module
       FROM canonical_capability c
       JOIN source_capability s ON s.canonical_id = c.id
      WHERE s.donor IN (${DONORS.map(() => '?').join(',')})`,
  )
  .all(...DONORS)

const sourceUpdate = db.prepare('UPDATE source_capability SET target_crate=? WHERE id=?')
const canonicalUpdate = db.prepare('UPDATE canonical_capability SET target_crate=? WHERE id=?')
const workExecutionModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-workflows',
         target_module='work_execution',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-workflows --test work_execution')
   WHERE key=?
`)
const statusOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-workflows --test work_execution', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const runtimeInvokeModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-runtime',
         target_module='invoke',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-workflows --test work_execution')
   WHERE key=?
`)
const runtimeInvokeOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-workflows --test work_execution', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const agentLoopSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-workflows',
         target_module='agent_loop'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const agentLoopModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-workflows',
         target_module='agent_loop',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-workflows --test agent_loop')
   WHERE key=?
`)
const agentLoopOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-workflows --test agent_loop', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const toolHandlerSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-integrations',
         target_module='tool_handlers'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const toolHandlerModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-integrations',
         target_module='tool_handlers',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-integrations --test tool_handlers')
   WHERE key=?
`)
const toolHandlerOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-integrations --test tool_handlers', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const mcpServersSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-integrations',
         target_module='mcp_servers'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const mcpServersModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-integrations',
         target_module='mcp_servers',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-integrations --test mcp_servers')
   WHERE key=?
`)
const mcpServersOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-integrations --test mcp_servers', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const integrationToolsSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-integrations',
         target_module='integration_tools'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const integrationToolsModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-integrations',
         target_module='integration_tools',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-integrations --test integration_tools')
   WHERE key=?
`)
const integrationToolsOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-integrations --test integration_tools', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const ptcSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-integrations',
         target_module='ptc'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const ptcModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-integrations',
         target_module='ptc',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-integrations --test ptc')
   WHERE key=?
`)
const ptcOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-integrations --test ptc', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const fileUploadsSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-memory',
         target_module='file_uploads'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const fileUploadsModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-memory',
         target_module='file_uploads',
         side_effect_class=COALESCE(NULLIF(side_effect_class, ''), 'internal_write'),
         requires_approval=0,
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-memory --test file_uploads')
   WHERE key=?
`)
const fileUploadsOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-memory --test file_uploads', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const workflowLifecycleSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-workflows',
         target_module='workflow_lifecycle'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const workflowLifecycleModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-workflows',
         target_module='workflow_lifecycle',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-workflows --test workflow_lifecycle')
   WHERE key=?
`)
const workflowLifecycleOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-workflows --test workflow_lifecycle', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const workflowTaskQueueSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-workflows',
         target_module='workflow_task_queue'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const workflowTaskQueueModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-workflows',
         target_module='workflow_task_queue',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-workflows --test workflow_task_queue')
   WHERE key=?
`)
const workflowTaskQueueOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-workflows --test workflow_task_queue', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const workflowReplicationDlqSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-workflows',
         target_module='workflow_task_queue'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const workflowReplicationDlqModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-workflows',
         target_module='workflow_task_queue',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test='cargo test -p chronica-workflows --test workflow_replication_dlq'
   WHERE key=?
`)
const workflowReplicationDlqOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-workflows --test workflow_replication_dlq', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=excluded.acceptance_test,
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const workflowDlqReplaySourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-workflows',
         target_module='workflow_dlq_replay'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const workflowDlqReplayModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-workflows',
         target_module='workflow_dlq_replay',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-workflows --test workflow_dlq_replay')
   WHERE key=?
`)
const workflowDlqReplayOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-workflows --test workflow_dlq_replay', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const partialExecutionSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-workflows',
         target_module='partial_execution'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const partialExecutionModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-workflows',
         target_module='partial_execution',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-workflows --test partial_execution')
   WHERE key=?
`)
const partialExecutionOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-workflows --test partial_execution', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const workflowControlFlowSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-workflows',
         target_module='workflow_control_flow'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const workflowControlFlowModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-workflows',
         target_module='workflow_control_flow',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-workflows --test workflow_control_flow'),
         blocker=NULL
   WHERE key=?
`)
const workflowControlFlowOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-workflows --test workflow_control_flow', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const executionDataSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-workflows',
         target_module='execution_data'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const executionDataModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-workflows',
         target_module='execution_data',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-workflows --test execution_data')
   WHERE key=?
`)
const executionDataOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-workflows --test execution_data', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const humanInputSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-workflows',
         target_module='human_input'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const humanInputModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-workflows',
         target_module='human_input',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-workflows --test human_input')
   WHERE key=?
`)
const humanInputOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-workflows --test human_input', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const humanApprovalQueueSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-workflows',
         target_module='human_approval_queue'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const humanApprovalQueueModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-workflows',
         target_module='human_approval_queue',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-workflows --test human_approval_queue')
   WHERE key=?
`)
const humanApprovalQueueOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-workflows --test human_approval_queue', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const workflowToolSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-workflows',
         target_module='workflow_tool'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const workflowToolModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-workflows',
         target_module='workflow_tool',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-workflows --test workflow_tool')
   WHERE key=?
`)
const workflowToolOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-workflows --test workflow_tool', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const workflowDispatchSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-workflows',
         target_module='workflow_dispatch'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const workflowDispatchModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-workflows',
         target_module='workflow_dispatch',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-workflows --test workflow_dispatch')
   WHERE key=?
`)
const workflowDispatchOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-workflows --test workflow_dispatch', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const externalWorkflowHonestyUpdate = db.prepare(`
  UPDATE canonical_capability
     SET status='unimplemented',
         target_crate='chronica-workflows',
         target_module=NULL,
         acceptance_test=NULL,
         blocker=COALESCE(blocker, 'native partial execution is coded; external n8n/vendor workflow invocation remains intentionally unimplemented')
   WHERE key='workflow-runtime.invoke_external_workflow'
     AND status!='verified'
`)
const externalWorkflowHonestyOverride = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,blocker,set_at,set_by)
  VALUES (
    'workflow-runtime.invoke_external_workflow',
    'unimplemented',
    NULL,
    'native partial execution is coded; external n8n/vendor workflow invocation remains intentionally unimplemented',
    ?,
    'codex:normalize-ai-work-targets'
  )
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=NULL,
    blocker=excluded.blocker,
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const knowledgeLifecycleSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-memory',
         target_module='knowledge_lifecycle'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const knowledgeLifecycleModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-memory',
         target_module='knowledge_lifecycle',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-memory --test knowledge_lifecycle')
   WHERE key=?
`)
const knowledgeLifecycleOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-memory --test knowledge_lifecycle', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const workflowScheduleSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-workflows',
         target_module='schedule'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const workflowScheduleModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-workflows',
         target_module='schedule',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-workflows schedule::')
   WHERE key=?
`)
const workflowScheduleOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-workflows schedule::', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const workflowVariableExtractionSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-workflows',
         target_module='workflow_variable_extraction'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const workflowVariableExtractionModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-workflows',
         target_module='workflow_variable_extraction',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-workflows --test workflow_variable_extraction')
   WHERE key=?
`)
const workflowVariableExtractionOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-workflows --test workflow_variable_extraction', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const workflowAppSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-workflows',
         target_module='workflow_app'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const workflowAppModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-workflows',
         target_module='workflow_app',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-workflows --test workflow_app')
   WHERE key=?
`)
const workflowAppOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-workflows --test workflow_app', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const workFeedSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-company',
         target_module='work_feed'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const workFeedModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-company',
         target_module='work_feed',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-company --test work_feed')
   WHERE key=?
`)
const workFeedOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-company --test work_feed', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const agentRunTraceSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-observability',
         target_module='agent_run_trace'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const agentRunTraceModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-observability',
         target_module='agent_run_trace',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-observability --test agent_run_trace')
   WHERE key=?
`)
const agentRunTraceOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-observability --test agent_run_trace', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const assistantConversationsSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-traces',
         target_module='ai_assistant_conversations'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const assistantConversationsModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-traces',
         target_module='ai_assistant_conversations',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-traces --test ai_assistant_conversations')
   WHERE key=?
`)
const assistantConversationsOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-traces --test ai_assistant_conversations', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const requestCompressionSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-observability',
         target_module='request_compression'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const requestCompressionModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-observability',
         target_module='request_compression',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-observability --test request_compression')
   WHERE key=?
`)
const requestCompressionOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-observability --test request_compression', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const logChunksSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-observability',
         target_module='log_chunks'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const logChunksModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-observability',
         target_module='log_chunks',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-observability --test log_chunks')
   WHERE key=?
`)
const logChunksOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-observability --test log_chunks', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const providerConfigSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-runtime',
         target_module='provider_config'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const providerConfigModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-runtime',
         target_module='provider_config',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-runtime --test provider_config')
   WHERE key=?
`)
const providerConfigOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-runtime --test provider_config', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const toolPermissionSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-runtime',
         target_module='tool_permissions'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const toolPermissionModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-runtime',
         target_module='tool_permissions',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-runtime --test tool_permissions')
   WHERE key=?
`)
const toolPermissionOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-runtime --test tool_permissions', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const toolCallResolverSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-runtime',
         target_module='tool_call_resolver'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const toolCallResolverModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-runtime',
         target_module='tool_call_resolver',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-runtime --test tool_call_resolver')
   WHERE key=?
`)
const toolCallResolverOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-runtime --test tool_call_resolver', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const secretScanSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-security',
         target_module='redact'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const secretScanModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-security',
         target_module='redact',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-security --test exposed_secrets')
   WHERE key=?
`)
const secretScanOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-security --test exposed_secrets', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const apiKeySourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-security',
         target_module='api_key'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const apiKeyModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-security',
         target_module='api_key',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-security --test api_key_resource')
   WHERE key=?
`)
const apiKeyOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-security --test api_key_resource', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const accessPermissionSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-security',
         target_module='access'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const accessPermissionModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-security',
         target_module='access',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-security --test access_check')
   WHERE key=?
`)
const accessPermissionOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-security --test access_check', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const distributedTraceSourceUpdate = db.prepare(`
  UPDATE source_capability
     SET target_crate='chronica-observability',
         target_module='trace'
   WHERE canonical_id=(SELECT id FROM canonical_capability WHERE key=?)
`)
const distributedTraceModuleUpdate = db.prepare(`
  UPDATE canonical_capability
     SET target_crate='chronica-observability',
         target_module='trace',
         status=CASE WHEN status='verified' THEN status ELSE 'implemented_unverified' END,
         acceptance_test=COALESCE(acceptance_test, 'cargo test -p chronica-observability --test distributed_tracing')
   WHERE key=?
`)
const distributedTraceOverrideUpsert = db.prepare(`
  INSERT INTO canonical_status_override (canonical_key,status,acceptance_test,set_at,set_by)
  VALUES (?, 'implemented_unverified', 'cargo test -p chronica-observability --test distributed_tracing', ?, 'codex:normalize-ai-work-targets')
  ON CONFLICT(canonical_key) DO UPDATE SET
    status=CASE WHEN canonical_status_override.status='verified' THEN canonical_status_override.status ELSE excluded.status END,
    acceptance_test=COALESCE(canonical_status_override.acceptance_test, excluded.acceptance_test),
    set_at=excluded.set_at,
    set_by=excluded.set_by
`)
const donorScopeUpdate = db.prepare('UPDATE donor_scope SET target_crates=? WHERE donor=?')

let sourceChanged = 0
let canonicalChanged = 0

const tx = db.transaction(() => {
  for (const row of sourceRows) {
    const normalized = normalizeTarget(row)
    if (normalized !== row.target_crate) {
      sourceUpdate.run(normalized, row.id)
      sourceChanged++
    }
  }

  for (const row of canonicalRows) {
    const normalized = normalizeTarget(row)
    if (normalized !== row.target_crate) {
      canonicalUpdate.run(normalized, row.id)
      canonicalChanged++
    }
  }

  donorScopeUpdate.run(
    'chronica-workflows, chronica-runtime, chronica-integrations, chronica-memory, chronica-security, chronica-observability, chronica-core, chronica-scheduler',
    'n8n-master',
  )
  donorScopeUpdate.run(
    'chronica-memory, chronica-workflows, chronica-runtime, chronica-integrations, chronica-security, chronica-observability, chronica-erp, chronica-media, chronica-core',
    'dify',
  )
  donorScopeUpdate.run(
    'chronica-workflows, chronica-runtime, chronica-integrations, chronica-memory, chronica-security, chronica-observability, chronica-template-store, chronica-media, chronica-core',
    'refly-main',
  )

  for (const key of [
    'workflow-runtime.manage_human_input',
    'agent-knowledge.tool_ask_user',
    'agent-knowledge.assemble_agent_context',
  ]) {
    workExecutionModuleUpdate.run(key)
    statusOverrideUpsert.run(key, now)
  }

  for (const key of ['agent.invoke_tool_call']) {
    runtimeInvokeModuleUpdate.run(key)
    runtimeInvokeOverrideUpsert.run(key, now)
  }

  for (const key of ['agent-knowledge.execute_agent_loop']) {
    agentLoopSourceUpdate.run(key)
    agentLoopModuleUpdate.run(key)
    agentLoopOverrideUpsert.run(key, now)
  }

  for (const key of ['agent-knowledge.run_tool_handler_pipeline']) {
    toolHandlerSourceUpdate.run(key)
    toolHandlerModuleUpdate.run(key)
    toolHandlerOverrideUpsert.run(key, now)
  }

  for (const key of ['agent-knowledge.manage_mcp_servers']) {
    mcpServersSourceUpdate.run(key)
    mcpServersModuleUpdate.run(key)
    mcpServersOverrideUpsert.run(key, now)
  }

  for (const key of ['agent-knowledge.provide_integration_tools']) {
    integrationToolsSourceUpdate.run(key)
    integrationToolsModuleUpdate.run(key)
    integrationToolsOverrideUpsert.run(key, now)
  }

  for (const key of ['agent-knowledge.execute_ptc_tool']) {
    ptcSourceUpdate.run(key)
    ptcModuleUpdate.run(key)
    ptcOverrideUpsert.run(key, now)
  }

  for (const key of ['agent-knowledge.manage_file_uploads']) {
    fileUploadsSourceUpdate.run(key)
    fileUploadsModuleUpdate.run(key)
    fileUploadsOverrideUpsert.run(key, now)
  }

  for (const key of [
    'workflow-runtime.manage_workflow_run_lifecycle',
    'workflow-runtime.queue_and_prioritize_execution',
    'workflow-runtime.recover_crashed_executions',
    'workflow-runtime.register_lifecycle_hooks',
  ]) {
    workflowLifecycleSourceUpdate.run(key)
    workflowLifecycleModuleUpdate.run(key)
    workflowLifecycleOverrideUpsert.run(key, now)
  }

  for (const key of [
    'workflow-runtime.manage_task_queue',
    'workflow-runtime.manage_dlq_tasks',
    'workflow-runtime.enforce_job_limits',
    'workflow-runtime.poll_job',
  ]) {
    workflowTaskQueueSourceUpdate.run(key)
    workflowTaskQueueModuleUpdate.run(key)
    workflowTaskQueueOverrideUpsert.run(key, now)
  }

  for (const key of ['workflow-runtime.manage_replication_dlq']) {
    workflowReplicationDlqSourceUpdate.run(key)
    workflowReplicationDlqModuleUpdate.run(key)
    workflowReplicationDlqOverrideUpsert.run(key, now)
  }

  for (const key of ['workflow-runtime.replay_dlq_messages']) {
    workflowDlqReplaySourceUpdate.run(key)
    workflowDlqReplayModuleUpdate.run(key)
    workflowDlqReplayOverrideUpsert.run(key, now)
  }

  for (const key of ['infra.model_workflow_execution_state']) {
    partialExecutionSourceUpdate.run(key)
    partialExecutionModuleUpdate.run(key)
    partialExecutionOverrideUpsert.run(key, now)
  }

  for (const key of [
    'workflow-runtime.conditional_branching',
    'workflow-runtime.loop_iteration',
    'workflow-runtime.parallel_execution',
  ]) {
    workflowControlFlowSourceUpdate.run(key)
    workflowControlFlowModuleUpdate.run(key)
    workflowControlFlowOverrideUpsert.run(key, now)
  }

  for (const key of ['data.compress']) {
    executionDataSourceUpdate.run(key)
    executionDataModuleUpdate.run(key)
    executionDataOverrideUpsert.run(key, now)
  }

  for (const key of [
    'workflow-runtime.ask_user_question',
    'workflow-runtime.execute_form_submission',
  ]) {
    humanInputSourceUpdate.run(key)
    humanInputModuleUpdate.run(key)
    humanInputOverrideUpsert.run(key, now)
  }

  for (const key of [
    'workflow-runtime.manage_human_approval_queue',
    'policy.approval.request_user_approval',
  ]) {
    humanApprovalQueueSourceUpdate.run(key)
    humanApprovalQueueModuleUpdate.run(key)
    humanApprovalQueueOverrideUpsert.run(key, now)
  }

  for (const key of [
    'workflow-runtime.provide_workflow_tools',
    'workflow-runtime.invoke_tool',
    'workflow-runtime.manage_tool_calls',
  ]) {
    workflowToolSourceUpdate.run(key)
    workflowToolModuleUpdate.run(key)
    workflowToolOverrideUpsert.run(key, now)
  }

  for (const key of [
    'workflow-runtime.manage_remote_triggers',
    'workflow-runtime.webhook_trigger',
    'workflow-runtime.schedule_cron_job',
  ]) {
    workflowDispatchSourceUpdate.run(key)
    workflowDispatchModuleUpdate.run(key)
    workflowDispatchOverrideUpsert.run(key, now)
  }

  for (const key of ['workflow-runtime.construct_cron_expression']) {
    workflowScheduleSourceUpdate.run(key)
    workflowScheduleModuleUpdate.run(key)
    workflowScheduleOverrideUpsert.run(key, now)
  }

  for (const key of ['workflow-runtime.extract_variables_from_prompt']) {
    workflowVariableExtractionSourceUpdate.run(key)
    workflowVariableExtractionModuleUpdate.run(key)
    workflowVariableExtractionOverrideUpsert.run(key, now)
  }

  externalWorkflowHonestyUpdate.run()
  externalWorkflowHonestyOverride.run(now)

  for (const key of [
    'dataset.manage',
    'agent-knowledge.embed_chunk_documents',
    'agent-knowledge.build_semantic_index',
    'agent-knowledge.serve_citation_docs',
    'agent-knowledge.sync_knowledge_document',
    'agent-knowledge.ingest_documents',
    'agent-knowledge.manage_rag_pipeline',
  ]) {
    knowledgeLifecycleSourceUpdate.run(key)
    knowledgeLifecycleModuleUpdate.run(key)
    knowledgeLifecycleOverrideUpsert.run(key, now)
  }

  for (const key of [
    'workflow-runtime.compute_canvas_graph_ops',
    'workflow-runtime.manage_canvas',
    'workflow-runtime.manage_apps',
    'workflow-runtime.run_workflow_app',
    'workflow-runtime.manage_sharing',
    'app.create',
    'app.export',
    'app.import',
  ]) {
    workflowAppSourceUpdate.run(key)
    workflowAppModuleUpdate.run(key)
    workflowAppOverrideUpsert.run(key, now)
  }

  for (const key of [
    'workflow-runtime.track_activity_feed',
    'comment.manage',
  ]) {
    workFeedSourceUpdate.run(key)
    workFeedModuleUpdate.run(key)
    workFeedOverrideUpsert.run(key, now)
  }

  for (const key of [
    'obs.trace_agent_runs',
    'obs.track_llm_token_usage',
    'erp.billing.calculate_cost',
    'erp.credit.track_usage',
  ]) {
    agentRunTraceSourceUpdate.run(key)
    agentRunTraceModuleUpdate.run(key)
    agentRunTraceOverrideUpsert.run(key, now)
  }

  for (const key of ['agent-knowledge.persist_assistant_conversations']) {
    assistantConversationsSourceUpdate.run(key)
    assistantConversationsModuleUpdate.run(key)
    assistantConversationsOverrideUpsert.run(key, now)
  }

  for (const key of ['obs.infra.compress_request']) {
    requestCompressionSourceUpdate.run(key)
    requestCompressionModuleUpdate.run(key)
    requestCompressionOverrideUpsert.run(key, now)
  }

  for (const key of ['obs.logs.store_log_chunks']) {
    logChunksSourceUpdate.run(key)
    logChunksModuleUpdate.run(key)
    logChunksOverrideUpsert.run(key, now)
  }

  for (const key of [
    'agent-knowledge.configure_llm_provider',
    'agent-knowledge.provide_llm_runtime_providers',
    'agent-knowledge.build_provider_clients',
    'agent-knowledge.check_provider_health',
    'agent-knowledge.validate_openai_credential',
  ]) {
    providerConfigSourceUpdate.run(key)
    providerConfigModuleUpdate.run(key)
    providerConfigOverrideUpsert.run(key, now)
  }

  for (const key of [
    'agent-knowledge.enforce_tool_permissions',
    'agent-knowledge.classify_assistant_action',
  ]) {
    toolPermissionSourceUpdate.run(key)
    toolPermissionModuleUpdate.run(key)
    toolPermissionOverrideUpsert.run(key, now)
  }

  for (const key of ['agent.tool_calls_resolve']) {
    toolCallResolverSourceUpdate.run(key)
    toolCallResolverModuleUpdate.run(key)
    toolCallResolverOverrideUpsert.run(key, now)
  }

  for (const key of ['policy.secrets.detect_exposed_secrets']) {
    secretScanSourceUpdate.run(key)
    secretScanModuleUpdate.run(key)
    secretScanOverrideUpsert.run(key, now)
  }

  for (const key of ['auth.apikey.create']) {
    apiKeySourceUpdate.run(key)
    apiKeyModuleUpdate.run(key)
    apiKeyOverrideUpsert.run(key, now)
  }

  for (const key of ['policy.access.check_permission']) {
    accessPermissionSourceUpdate.run(key)
    accessPermissionModuleUpdate.run(key)
    accessPermissionOverrideUpsert.run(key, now)
  }

  for (const key of ['obs.traces.emit_distributed_tracing']) {
    distributedTraceSourceUpdate.run(key)
    distributedTraceModuleUpdate.run(key)
    distributedTraceOverrideUpsert.run(key, now)
  }

  db.prepare("INSERT OR REPLACE INTO meta(k,v) VALUES('ai_work_donor_targets_normalized_at', ?)").run(now)
  db.prepare("INSERT OR REPLACE INTO meta(k,v) VALUES('ai_work_donor_targets_normalized_donors', ?)").run(DONORS.join(','))
})

tx()

const invalid = db
  .prepare(
    `SELECT target_crate, COUNT(*) AS caps
       FROM canonical_capability c
      WHERE c.id IN (
        SELECT DISTINCT canonical_id FROM source_capability
         WHERE donor IN (${DONORS.map(() => '?').join(',')})
      )
        AND target_crate NOT IN (${Array.from(KNOWN_CRATES).map(() => '?').join(',')})
      GROUP BY target_crate
      ORDER BY caps DESC`,
  )
  .all(...DONORS, ...KNOWN_CRATES)

console.log('normalize-ai-work-donor-targets:')
console.log(`  source rows changed: ${sourceChanged}/${sourceRows.length}`)
console.log(`  canonical rows changed: ${canonicalChanged}/${canonicalRows.length}`)
console.log(`  invalid canonical targets remaining: ${invalid.length}`)
if (invalid.length) {
  for (const row of invalid) console.log(`    ${row.target_crate || '(blank)'}: ${row.caps}`)
}

db.close()
