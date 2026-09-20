// Deterministic money-free reactive NewsApp fixture for Standard 205 §59–64.
// Models CAP12 handle / CAP13 verify+dedupe / CAP15 typed refresh / domain normalize.
// Not a product runtime. Agents query canonical state; they do not scrape the fixture DOM
// or read the external store as Chronica truth.

export const PREMIUM_SECRET_CANARY = 'SECRET_CANARY_PREMIUM_do_not_leak'
export const FREE_SECRET_CANARY = 'SECRET_CANARY_FREE_do_not_leak'
export const NEWS_SOURCE_ID = 'fixture.news.reactive'

function nowIso() {
  return '2026-08-21T12:00:00.000Z'
}

function assertNoSecret(value, label) {
  const blob = typeof value === 'string' ? value : JSON.stringify(value)
  if (blob.includes(PREMIUM_SECRET_CANARY) || blob.includes(FREE_SECRET_CANARY)) {
    throw new Error(`credential leaked into ${label}`)
  }
}

function toNewsEvent(article, { receivedAt, sourceUrl }) {
  return {
    type: 'NewsEvent',
    id: `news:${article.event_id}`,
    headline: article.title,
    summary: article.body,
    premium: article.premium === true,
    source: NEWS_SOURCE_ID,
    webapp: 'NewsApp',
    integration: 'fixture-news',
    entitlement: article.premium ? 'premium' : 'free',
    provider_event_id: article.event_id,
    occurred_at: article.provider_timestamp,
    received_at: receivedAt,
    source_url: sourceUrl,
    evidence_ref: article.premium ? 'premium-content:metadata-only' : `fixture:${article.id}`,
  }
}

export function createReactiveNewsFixture(snapshot = null) {
  const external = snapshot?.external
    ? structuredClone(snapshot.external)
    : {
        version: 1,
        etag: 'etag-v1',
        last_modified: '2026-08-21T00:00:00Z',
        articles: [
          {
            id: 'art-1',
            title: 'Markets open',
            body: 'Public lede.',
            premium: false,
            event_id: 'evt-1',
            provider_timestamp: '2026-08-21T00:00:00Z',
          },
        ],
      }

  const durable = snapshot?.durable
    ? {
        checkpoint: { ...snapshot.durable.checkpoint },
        dedupe: new Set(snapshot.durable.dedupe || []),
        canonical: structuredClone(snapshot.durable.canonical),
        source_health: snapshot.durable.source_health,
        logs: structuredClone(snapshot.durable.logs || []),
        receipts: structuredClone(snapshot.durable.receipts || []),
      }
    : {
        checkpoint: { last_version: 1, last_etag: 'etag-v1', last_event_id: 'evt-1' },
        dedupe: new Set(['news:evt-1']),
        canonical: [
          toNewsEvent(external.articles[0], {
            receivedAt: nowIso(),
            sourceUrl: 'https://fixture.news/art-1',
          }),
        ],
        source_health: 'CONNECTED',
        logs: [],
        receipts: [],
      }

  const credentials = new Map(
    snapshot?.credentials
      ? snapshot.credentials
      : [
          [
            'cred_premium',
            {
              tenant_id: 'tenant-a',
              owner: 'company:tenant-a',
              entitlement: 'premium',
              secret: PREMIUM_SECRET_CANARY,
              revoked: false,
            },
          ],
          [
            'cred_free',
            {
              tenant_id: 'tenant-a',
              owner: 'company:tenant-a',
              entitlement: 'free',
              secret: FREE_SECRET_CANARY,
              revoked: false,
            },
          ],
        ],
  )

  function log(msg, extra = {}) {
    const row = { msg, at: nowIso(), ...extra }
    assertNoSecret(row, 'logs')
    durable.logs.push(row)
  }

  function receipt(kind, extra = {}) {
    const row = { kind, at: nowIso(), ...extra }
    assertNoSecret(row, 'receipts')
    durable.receipts.push(row)
  }

  function getCredential(handle) {
    const cred = credentials.get(handle)
    if (!cred) {
      durable.source_health = 'AUTH_EXPIRED'
      throw new Error('unknown credential_handle')
    }
    if (cred.revoked) {
      durable.source_health = 'AUTH_EXPIRED'
      throw new Error('credential revoked')
    }
    return cred
  }

  function latestById() {
    const map = new Map()
    for (const article of external.articles) map.set(article.id, article)
    return [...map.values()]
  }

  function targetedRefresh({ sinceVersion = 0 } = {}) {
    const added = latestById().filter((article) => {
      const versionHint = Number(String(article.event_id).replace(/\D/g, '')) || 0
      return versionHint > sinceVersion || !durable.canonical.some((row) => row.provider_event_id === article.event_id)
    })
    const normalized = []
    for (const article of added) {
      const key = `news:${article.event_id}`
      if (durable.dedupe.has(key)) continue
      durable.dedupe.add(key)
      const event = toNewsEvent(article, {
        receivedAt: nowIso(),
        sourceUrl: `https://fixture.news/${article.id}`,
      })
      durable.canonical.push(event)
      normalized.push(event)
    }
    durable.checkpoint = {
      last_version: external.version,
      last_etag: external.etag,
      last_event_id: external.articles.at(-1)?.event_id || durable.checkpoint.last_event_id,
    }
    durable.source_health = 'CONNECTED'
    return normalized
  }

  function publishUpdate(article) {
    external.version += 1
    external.etag = `etag-v${external.version}`
    external.last_modified = nowIso()
    external.articles.push(article)
    return {
      webhook: {
        event_id: article.event_id,
        signal: 'article_updated',
        version: external.version,
        incomplete: true,
      },
    }
  }

  function ingestWebhook({ event_id, signature_ok, credential_handle = 'cred_premium' }) {
    getCredential(credential_handle)
    if (signature_ok !== true) {
      log('webhook rejected', { reason: 'signature' })
      throw new Error('webhook signature absent')
    }
    const key = `cap13:${event_id}`
    if (durable.dedupe.has(key)) {
      log('webhook duplicate ignored', { event_id })
      receipt('inbound_duplicate_ignored', { event_id })
      return { outcome: 'duplicate_ignored', canonical_added: 0 }
    }
    durable.dedupe.add(key)
    log('webhook accepted as signal', { event_id })
    const added = targetedRefresh({ sinceVersion: durable.checkpoint.last_version })
    receipt('inbound_reconciled', { event_id, added: added.length })
    return { outcome: 'reconciled', canonical_added: added.length, events: added }
  }

  function poll() {
    if (durable.checkpoint.last_etag === external.etag && durable.checkpoint.last_version === external.version) {
      log('poll no-change', { etag: external.etag })
      return { changed: false, canonical_added: 0 }
    }
    const added = targetedRefresh({ sinceVersion: durable.checkpoint.last_version })
    log('poll reconciled', { etag: external.etag, added: added.length })
    return { changed: true, canonical_added: added.length, events: added }
  }

  function invokeWebapp(method, { credential_handle, article_id } = {}) {
    const cred = getCredential(credential_handle)
    if (method === 'search_news') {
      return latestById()
        .filter((article) => !article.premium || cred.entitlement === 'premium')
        .map((article) => ({ id: article.id, title: article.title, premium: article.premium }))
    }
    if (method === 'get_article') {
      const article = latestById().find((row) => row.id === article_id)
      if (!article) throw new Error('not found')
      if (article.premium && cred.entitlement !== 'premium') {
        durable.source_health = 'ENTITLEMENT_MISSING'
        throw new Error('entitlement missing')
      }
      return { id: article.id, title: article.title, body: article.body, premium: article.premium }
    }
    if (method === 'refresh') {
      return poll()
    }
    throw new Error(`unknown method ${method}`)
  }

  function revoke(handle) {
    const cred = credentials.get(handle)
    if (cred) cred.revoked = true
    log('credential revoked', { credential_handle: handle })
  }

  function agentToolSchema() {
    const schema = {
      app: 'NewsApp',
      methods: ['search_news', 'get_article', 'refresh', 'watch_news_topic'],
      access_state: durable.source_health,
      credential_handle: 'cred_premium',
    }
    assertNoSecret(schema, 'agent tool schema')
    return schema
  }

  function agentQuery() {
    const pack = {
      source: NEWS_SOURCE_ID,
      source_health: durable.source_health,
      observed_at: nowIso(),
      freshness: durable.source_health === 'CONNECTED' ? 'POLL_INTERVAL' : durable.source_health,
      events: durable.canonical.map((row) => ({
        type: row.type,
        id: row.id,
        headline: row.headline,
        occurred_at: row.occurred_at,
        received_at: row.received_at,
        source: row.source,
        provider_event_id: row.provider_event_id,
      })),
    }
    assertNoSecret(pack, 'agent context')
    return pack
  }

  function leakSurfaces() {
    return {
      logs: durable.logs,
      receipts: durable.receipts,
      agent_tool_schema: agentToolSchema(),
      agent_context: agentQuery(),
      app_definition: {
        app: 'NewsApp',
        credential_handle: 'cred_premium',
        entitlement: 'premium',
      },
    }
  }

  function snapshotState() {
    return {
      external: structuredClone(external),
      durable: {
        checkpoint: { ...durable.checkpoint },
        dedupe: [...durable.dedupe],
        canonical: structuredClone(durable.canonical),
        source_health: durable.source_health,
        logs: structuredClone(durable.logs),
        receipts: structuredClone(durable.receipts),
      },
      credentials: [...credentials.entries()].map(([handle, cred]) => [handle, { ...cred }]),
    }
  }

  function restart() {
    return createReactiveNewsFixture(snapshotState())
  }

  return {
    NEWS_SOURCE_ID,
    publishUpdate,
    ingestWebhook,
    poll,
    invokeWebapp,
    revoke,
    agentToolSchema,
    agentQuery,
    leakSurfaces,
    restart,
    snapshotState,
    checkpoint: () => ({ ...durable.checkpoint }),
    canonical: () => durable.canonical.slice(),
    sourceHealth: () => durable.source_health,
    externalVersion: () => external.version,
  }
}
