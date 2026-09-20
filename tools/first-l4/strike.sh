#!/usr/bin/env bash
# First-L4 strike — drive the real money chain over HTTP against the LIVE server (dev-auth bridge).
# The path the whole Planning Brain converged on. A REAL human runs this: REQ_ACTOR is a real person
# (requester), BOARD_ACTOR is a real board member (approver), and they MUST differ (Separation of Duties).
#
# Order is code-verified against companies.rs (money_action_pre_approved) + approvals.rs (create/approve):
#   create a BOUND money_action approval  ->  BOARD approves it  ->  post the money action WITH its id.
# (The money-action route does NOT mint an approval; it CONSUMES a board-approved, bound, single-use one.)
#
# Prereqs (see README.md): dev PG on :55432 ; psql -f seed.sql ; server booted with CHRONICA_DEV_AUTH=1 ;
# export BASE_URL to the address it logs.
#
# Honesty: a dev-auth + seeded run with a real human at the keyboard is the first L4-CANDIDATE (real human
# in the loop, full chain, real audit row; synthetic $ + seeded company) — the first crack in the 0%.
set -euo pipefail

BASE="${BASE_URL:-http://127.0.0.1:8080}"
ACCT="${ACCT:-acct-first-l4}"
CID="${CID:-f1f1f1f1-0000-4000-8000-000000000001}"
REQ_ACTOR="${REQ_ACTOR:?set REQ_ACTOR to the REAL requester identity, e.g. your-name}"
BOARD_ACTOR="${BOARD_ACTOR:?set BOARD_ACTOR to the REAL board approver (MUST differ from REQ_ACTOR — SoD)}"
NONCE="first-l4-$(date +%s)"
post() { curl -fsS "$BASE$1" -X POST -H 'content-type: application/json' "${@:2}"; }

echo "== 1) requester logs in (real human) =="
TOK_U=$(post /api/dev/login -d "{\"account_id\":\"$ACCT\",\"actor_id\":\"$REQ_ACTOR\",\"actor_type\":\"user\"}" | jq -r .token)

echo "== 2) requester CREATES the bound money_action approval (binds provider+operation+reversible+maxCostCents) =="
APPROVAL=$(post "/api/companies/$CID/approvals" -H "authorization: Bearer $TOK_U" \
  -d '{"type":"money_action","payload":{"provider":"stripe","operation":"payout","reversible":false,"maxCostCents":100}}')
echo "   $APPROVAL"
AID=$(echo "$APPROVAL" | jq -r '.id')
[ -n "$AID" ] && [ "$AID" != "null" ] || { echo "no approval id in create response — inspect \$APPROVAL"; exit 3; }

echo "== 3) board logs in (DIFFERENT actor → SoD) and APPROVES =="
TOK_B=$(post /api/dev/login -d "{\"account_id\":\"$ACCT\",\"actor_id\":\"$BOARD_ACTOR\",\"actor_type\":\"board\"}" | jq -r .token)
post "/api/approvals/$AID/approve" -H "authorization: Bearer $TOK_B" -d '{"decisionNote":"first-L4 strike approval"}'

echo "== 4) requester posts the \$1 money action WITH the approval → commits EXACTLY ONCE + Merkle audit row =="
post "/api/companies/$CID/money-actions" -H "authorization: Bearer $TOK_U" \
  -d "{\"provider\":\"stripe\",\"operation\":\"payout\",\"estimatedCostCents\":100,\"reversible\":false,\"nonce\":\"$NONCE\",\"approvalRequestId\":\"$AID\"}"

echo; echo "== 5) read the committed finance_event — the AUDIT EVIDENCE =="
curl -fsS "$BASE/api/companies/$CID/erp-events" -H "authorization: Bearer $TOK_U"
echo
echo "== DONE — if step 4 returned status:committed and step 5 shows a finance_event, the first L4-candidate ran. Record via README step 6. =="
