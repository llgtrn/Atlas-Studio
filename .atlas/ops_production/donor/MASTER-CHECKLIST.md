---
id: ops-production-donor-master-checklist
type: reference
status: active
canonical: true
---
# Donor Corpus Master Checklist (generated)

> **Generated from `tools/refoundation/donor-corpus.yaml` by `node tools/refoundation/donor-corpus.mjs --report`. Do not hand-edit this file — edit the YAML checklist and regenerate.**

This checklist is repository governance metadata (CHRONICA CORPUS MASTER CHECKLIST directive, 2026-09-16), not canonical runtime truth. It answers what donor repositories exist, which are relevant, which have source present, and which have been censused/absorbed/retired. It does not sequence donor execution order and does not cover licensing (see `license/donors/` and `docs/ops_production/donor/DONOR-CENSUS-PROVENANCE.md` for that).

Generated at: 2026-09-16T15:30:38.434Z

## Source evidence

- temporary/manifest.yaml (90 already-ingested donors, real upstream_sha provenance)
- tools/repo/donors.manifest (81-entry pre-existing reference manifest)
- user-provided round-2 expansion list (212 rows, categorized: web2app/CRM/OSINT/PlatformOps/FinOps/Robotics/Industrial/Edge/Fleet/Simulation/Legal/Banking/Trading/Estate/etc.)
- docs/ and tools/ existing scattered github.com references (Ops census docs, capability census JSON, prior manifests)

## Totals

- **Total unique donors:** 323
- **Duplicates collapsed:** 0

## By status

| Status | Count |
|---|---|
| DISCOVERED | 221 |
| NEEDED | 0 |
| REVIEW_NEEDED | 0 |
| SOURCE_PRESENT | 68 |
| CENSUSED | 0 |
| ABSORBING | 33 |
| PARTIALLY_ABSORBED | 1 |
| ABSORBED | 0 |
| RETIRE_SOURCE | 0 |
| RETIRED | 0 |
| REFERENCE_ONLY | 0 |
| REDUNDANT | 0 |
| DUPLICATE | 0 |
| NOT_NEEDED | 0 |

## By family

| Family | Count |
|---|---|
| AI_RUNTIME | 27 |
| OSINT | 25 |
| PLATFORM_ENGINEERING | 23 |
| DURABLE_STATE | 22 |
| OBSERVABILITY | 20 |
| COMMERCE | 15 |
| ROBOTICS | 14 |
| CUSTOMER | 12 |
| SERVERLESS | 10 |
| WEB2APP | 10 |
| HOSPITALITY | 10 |
| CLOUD | 9 |
| LOGISTICS | 9 |
| AUTHORITY | 8 |
| MODEL_SERVING | 8 |
| EDGE | 8 |
| TRADING | 8 |
| FINANCE | 7 |
| INDUSTRIAL | 7 |
| LEGAL | 7 |
| TRADE | 6 |
| CONTAINER | 6 |
| EXECUTION | 5 |
| BUILDING | 5 |
| NETWORK | 5 |
| DOCUMENT | 5 |
| ESTATE | 5 |
| EVENTING | 4 |
| IDENTITY | 4 |
| MEMORY | 4 |
| OTHER | 4 |
| SIMULATION | 3 |
| BANKING | 3 |
| SECURITY | 2 |
| FLEET | 2 |
| DEVELOPER_TOOLING | 1 |

## Absorption burn-down (live, computed from Git/filesystem at generation time)

This section is never hand-edited and never persisted as static data -- it is recomputed by `node tools/refoundation/donor-burndown.mjs --summary` every time this report regenerates. See that tool for per-donor detail (`--donor <slug>`) and the stall/doc-churn detectors (`--stalled`, `--doc-churn`).

```text
CHRONICA DONOR CORPUS

Unique donors              323

Corpus classified          323 / 323     100.00%
Evaluated (status set)     102 / 323     31.58%

Source present             39
Functional absorption started  0
Source drain not started    26
Source drain active         0
Fully drained               13
Retired (status)            0

Source baseline files      58410
Source remaining files     0
Source files drained       58410

SOURCE FILE DRAIN PROGRESS   100.00%
```

## Full donor list

| ID | Repo | Family | Status | Needed | Source present | Absorption state |
|---|---|---|---|---|---|---|
| D001 | [activepieces/activepieces](https://github.com/activepieces/activepieces) | CUSTOMER | SOURCE_PRESENT | UNKNOWN | no | none |
| D002 | [adbar/trafilatura](https://github.com/adbar/trafilatura) | OSINT | DISCOVERED | UNKNOWN | no | none |
| D003 | [AIDC-AI/Pixelle-Video](https://github.com/AIDC-AI/Pixelle-Video) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | no | none |
| D004 | [akeneo/pim-community-dev](https://github.com/akeneo/pim-community-dev) | COMMERCE | DISCOVERED | UNKNOWN | no | none |
| D005 | [alephdata/aleph](https://github.com/alephdata/aleph) | OSINT | DISCOVERED | UNKNOWN | no | none |
| D006 | [All-Hands-AI/OpenHands](https://github.com/All-Hands-AI/OpenHands) | AI_RUNTIME | DISCOVERED | UNKNOWN | no | none |
| D007 | [apache/age](https://github.com/apache/age) | DURABLE_STATE | ABSORBING | YES | yes | partial |
| D008 | [apache/airflow](https://github.com/apache/airflow) | SERVERLESS | DISCOVERED | UNKNOWN | no | none |
| D009 | [apache/camel](https://github.com/apache/camel) | TRADE | DISCOVERED | UNKNOWN | no | none |
| D010 | [apache/cloudstack](https://github.com/apache/cloudstack) | CLOUD | DISCOVERED | UNKNOWN | no | none |
| D011 | [apache/flink](https://github.com/apache/flink) | EXECUTION | SOURCE_PRESENT | UNKNOWN | no | none |
| D012 | [apache/hudi](https://github.com/apache/hudi) | DURABLE_STATE | DISCOVERED | UNKNOWN | no | none |
| D013 | [apache/iceberg](https://github.com/apache/iceberg) | DURABLE_STATE | DISCOVERED | UNKNOWN | no | none |
| D014 | [apache/kafka](https://github.com/apache/kafka) | EXECUTION | ABSORBING | YES | yes | partial |
| D015 | [apache/pulsar](https://github.com/apache/pulsar) | EVENTING | DISCOVERED | UNKNOWN | no | none |
| D016 | [apache/tika](https://github.com/apache/tika) | OSINT | DISCOVERED | UNKNOWN | no | none |
| D017 | [apify/crawlee](https://github.com/apify/crawlee) | WEB2APP | SOURCE_PRESENT | UNKNOWN | no | none |
| D018 | [appsmithorg/appsmith](https://github.com/appsmithorg/appsmith) | CUSTOMER | SOURCE_PRESENT | UNKNOWN | no | none |
| D019 | [aquasecurity/tracee](https://github.com/aquasecurity/tracee) | SECURITY | DISCOVERED | UNKNOWN | no | none |
| D020 | [aquasecurity/trivy](https://github.com/aquasecurity/trivy) | SECURITY | DISCOVERED | UNKNOWN | no | none |
| D021 | [ArduPilot/ardupilot](https://github.com/ArduPilot/ardupilot) | ROBOTICS | DISCOVERED | UNKNOWN | no | none |
| D022 | [argoproj/argo-cd](https://github.com/argoproj/argo-cd) | PLATFORM_ENGINEERING | SOURCE_PRESENT | UNKNOWN | no | none |
| D023 | [argoproj/argo-workflows](https://github.com/argoproj/argo-workflows) | SERVERLESS | DISCOVERED | UNKNOWN | no | none |
| D024 | [authzed/spicedb](https://github.com/authzed/spicedb) | AUTHORITY | ABSORBING | YES | yes | partial |
| D025 | [AutowareFoundation/autoware_universe](https://github.com/AutowareFoundation/autoware_universe) | ROBOTICS | DISCOVERED | UNKNOWN | no | none |
| D026 | [awslabs/gluonts](https://github.com/awslabs/gluonts) | HOSPITALITY | DISCOVERED | UNKNOWN | no | none |
| D027 | [backstage/backstage](https://github.com/backstage/backstage) | PLATFORM_ENGINEERING | DISCOVERED | UNKNOWN | no | none |
| D028 | [bacnet-stack/bacnet-stack](https://github.com/bacnet-stack/bacnet-stack) | BUILDING | DISCOVERED | UNKNOWN | no | none |
| D029 | [bagisto/bagisto](https://github.com/bagisto/bagisto) | COMMERCE | DISCOVERED | UNKNOWN | no | none |
| D030 | [baserow/baserow](https://github.com/baserow/baserow) | CUSTOMER | SOURCE_PRESENT | UNKNOWN | no | none |
| D031 | [beancount/beancount](https://github.com/beancount/beancount) | FINANCE | DISCOVERED | UNKNOWN | no | none |
| D032 | [bentoml/BentoML](https://github.com/bentoml/BentoML) | MODEL_SERVING | DISCOVERED | UNKNOWN | no | none |
| D033 | [browser-use/browser-use](https://github.com/browser-use/browser-use) | WEB2APP | SOURCE_PRESENT | UNKNOWN | no | none |
| D034 | [browserbase/stagehand](https://github.com/browserbase/stagehand) | WEB2APP | SOURCE_PRESENT | UNKNOWN | no | none |
| D035 | [bytecodealliance/wasmtime](https://github.com/bytecodealliance/wasmtime) | CONTAINER | DISCOVERED | UNKNOWN | no | none |
| D036 | [bytedance/deer-flow](https://github.com/bytedance/deer-flow) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | no | none |
| D037 | [calcom/cal.com](https://github.com/calcom/cal.com) | HOSPITALITY | DISCOVERED | UNKNOWN | no | none |
| D038 | [CANopenNode/CANopenNode](https://github.com/CANopenNode/CANopenNode) | INDUSTRIAL | DISCOVERED | UNKNOWN | no | none |
| D039 | [ceph/ceph](https://github.com/ceph/ceph) | PLATFORM_ENGINEERING | SOURCE_PRESENT | UNKNOWN | no | none |
| D040 | [cert-manager/cert-manager](https://github.com/cert-manager/cert-manager) | IDENTITY | DISCOVERED | UNKNOWN | no | none |
| D041 | [chatwoot/chatwoot](https://github.com/chatwoot/chatwoot) | CUSTOMER | DISCOVERED | UNKNOWN | no | none |
| D042 | [cilium/cilium](https://github.com/cilium/cilium) | PLATFORM_ENGINEERING | ABSORBING | YES | yes | partial |
| D043 | [cilium/tetragon](https://github.com/cilium/tetragon) | NETWORK | DISCOVERED | UNKNOWN | no | none |
| D044 | [ClickHouse/ClickHouse](https://github.com/ClickHouse/ClickHouse) | DURABLE_STATE | SOURCE_PRESENT | UNKNOWN | no | none |
| D045 | [cloud-hypervisor/cloud-hypervisor](https://github.com/cloud-hypervisor/cloud-hypervisor) | CONTAINER | DISCOVERED | UNKNOWN | no | none |
| D046 | [cloudnative-pg/cloudnative-pg](https://github.com/cloudnative-pg/cloudnative-pg) | DURABLE_STATE | DISCOVERED | UNKNOWN | no | none |
| D047 | [comfyanonymous/ComfyUI](https://github.com/comfyanonymous/ComfyUI) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | no | none |
| D048 | [containerd/containerd](https://github.com/containerd/containerd) | PLATFORM_ENGINEERING | ABSORBING | YES | yes | partial |
| D049 | [containers/podman-desktop](https://github.com/containers/podman-desktop) | PLATFORM_ENGINEERING | SOURCE_PRESENT | UNKNOWN | no | none |
| D050 | [coollabsio/coolify](https://github.com/coollabsio/coolify) | PLATFORM_ENGINEERING | SOURCE_PRESENT | UNKNOWN | no | none |
| D051 | [coredns/coredns](https://github.com/coredns/coredns) | PLATFORM_ENGINEERING | ABSORBING | YES | yes | partial |
| D052 | [coroot/coroot](https://github.com/coroot/coroot) | OBSERVABILITY | DISCOVERED | UNKNOWN | no | none |
| D053 | [cortezaproject/corteza](https://github.com/cortezaproject/corteza) | CUSTOMER | SOURCE_PRESENT | UNKNOWN | no | none |
| D054 | [crewAIInc/crewAI](https://github.com/crewAIInc/crewAI) | AI_RUNTIME | DISCOVERED | UNKNOWN | no | none |
| D055 | [crossplane/crossplane](https://github.com/crossplane/crossplane) | PLATFORM_ENGINEERING | SOURCE_PRESENT | UNKNOWN | no | none |
| D056 | [D4Vinci/Scrapling](https://github.com/D4Vinci/Scrapling) | WEB2APP | DISCOVERED | UNKNOWN | no | none |
| D057 | [dagger/dagger](https://github.com/dagger/dagger) | MODEL_SERVING | DISCOVERED | UNKNOWN | no | none |
| D058 | [dagster-io/dagster](https://github.com/dagster-io/dagster) | SERVERLESS | DISCOVERED | UNKNOWN | no | none |
| D059 | [dapr/dapr](https://github.com/dapr/dapr) | PLATFORM_ENGINEERING | DISCOVERED | UNKNOWN | no | none |
| D060 | [daytonaio/daytona](https://github.com/daytonaio/daytona) | PLATFORM_ENGINEERING | SOURCE_PRESENT | UNKNOWN | no | none |
| D061 | [dbt-labs/dbt-core](https://github.com/dbt-labs/dbt-core) | DURABLE_STATE | DISCOVERED | UNKNOWN | no | none |
| D062 | [debezium/debezium](https://github.com/debezium/debezium) | TRADE | DISCOVERED | UNKNOWN | no | none |
| D063 | [decolua/9router](https://github.com/decolua/9router) | WEB2APP | DISCOVERED | UNKNOWN | no | none |
| D064 | [deepset-ai/haystack](https://github.com/deepset-ai/haystack) | MEMORY | DISCOVERED | UNKNOWN | no | none |
| D065 | [delta-io/delta](https://github.com/delta-io/delta) | DURABLE_STATE | DISCOVERED | UNKNOWN | no | none |
| D066 | [dgtlmoon/changedetection.io](https://github.com/dgtlmoon/changedetection.io) | OSINT | DISCOVERED | UNKNOWN | no | none |
| D067 | [digital-druid/hoteldruid](https://github.com/digital-druid/hoteldruid) | HOSPITALITY | DISCOVERED | UNKNOWN | no | none |
| D068 | [directus/directus](https://github.com/directus/directus) | CUSTOMER | SOURCE_PRESENT | UNKNOWN | no | none |
| D069 | [docassemble/docassemble](https://github.com/docassemble/docassemble) | LEGAL | DISCOVERED | UNKNOWN | no | none |
| D070 | [documenso/documenso](https://github.com/documenso/documenso) | DOCUMENT | DISCOVERED | UNKNOWN | no | none |
| D071 | [duckdb/duckdb](https://github.com/duckdb/duckdb) | DURABLE_STATE | DISCOVERED | UNKNOWN | no | none |
| D072 | [e2b-dev/E2B](https://github.com/e2b-dev/E2B) | MODEL_SERVING | DISCOVERED | UNKNOWN | no | none |
| D073 | [eclipse-cyclonedds/cyclonedds](https://github.com/eclipse-cyclonedds/cyclonedds) | ROBOTICS | DISCOVERED | UNKNOWN | no | none |
| D074 | [eclipse-ditto/ditto](https://github.com/eclipse-ditto/ditto) | EDGE | DISCOVERED | UNKNOWN | no | none |
| D075 | [eclipse-hono/hono](https://github.com/eclipse-hono/hono) | EDGE | DISCOVERED | UNKNOWN | no | none |
| D076 | [eclipse-kanto/kanto](https://github.com/eclipse-kanto/kanto) | EDGE | DISCOVERED | UNKNOWN | no | none |
| D077 | [eclipse-mosquitto/mosquitto](https://github.com/eclipse-mosquitto/mosquitto) | INDUSTRIAL | DISCOVERED | UNKNOWN | no | none |
| D078 | [eclipse-sumo/sumo](https://github.com/eclipse-sumo/sumo) | SIMULATION | DISCOVERED | UNKNOWN | no | none |
| D079 | [eclipse-tahu/tahu](https://github.com/eclipse-tahu/tahu) | INDUSTRIAL | DISCOVERED | UNKNOWN | no | none |
| D080 | [eclipse-zenoh/zenoh](https://github.com/eclipse-zenoh/zenoh) | ROBOTICS | DISCOVERED | UNKNOWN | no | none |
| D081 | [edgexfoundry/edgex-go](https://github.com/edgexfoundry/edgex-go) | EDGE | DISCOVERED | UNKNOWN | no | none |
| D082 | [elceef/dnstwist](https://github.com/elceef/dnstwist) | OSINT | ABSORBING | YES | yes | partial |
| D083 | [emqx/emqx](https://github.com/emqx/emqx) | INDUSTRIAL | DISCOVERED | UNKNOWN | no | none |
| D084 | [envoyproxy/envoy](https://github.com/envoyproxy/envoy) | PLATFORM_ENGINEERING | ABSORBING | YES | yes | partial |
| D085 | [envoyproxy/gateway](https://github.com/envoyproxy/gateway) | NETWORK | DISCOVERED | UNKNOWN | no | none |
| D086 | [eProsima/Fast-DDS](https://github.com/eProsima/Fast-DDS) | ROBOTICS | DISCOVERED | UNKNOWN | no | none |
| D087 | [esig/dss](https://github.com/esig/dss) | LEGAL | DISCOVERED | UNKNOWN | no | none |
| D088 | [espocrm/espocrm](https://github.com/espocrm/espocrm) | CUSTOMER | SOURCE_PRESENT | UNKNOWN | no | none |
| D089 | [facebook/prophet](https://github.com/facebook/prophet) | HOSPITALITY | DISCOVERED | UNKNOWN | no | none |
| D090 | [falcosecurity/falco](https://github.com/falcosecurity/falco) | AUTHORITY | SOURCE_PRESENT | UNKNOWN | no | none |
| D091 | [fermyon/spin](https://github.com/fermyon/spin) | CONTAINER | DISCOVERED | UNKNOWN | no | none |
| D092 | [Fincept-Corporation/FinceptTerminal](https://github.com/Fincept-Corporation/FinceptTerminal) | TRADING | DISCOVERED | UNKNOWN | no | none |
| D093 | [firecracker-microvm/firecracker](https://github.com/firecracker-microvm/firecracker) | PLATFORM_ENGINEERING | ABSORBING | YES | yes | partial |
| D094 | [flatcar/Flatcar](https://github.com/flatcar/Flatcar) | CLOUD | DISCOVERED | UNKNOWN | no | none |
| D095 | [fluent/fluent-bit](https://github.com/fluent/fluent-bit) | OBSERVABILITY | DISCOVERED | UNKNOWN | no | none |
| D096 | [formbricks/formbricks](https://github.com/formbricks/formbricks) | HOSPITALITY | DISCOVERED | UNKNOWN | no | none |
| D097 | [frappe/crm](https://github.com/frappe/crm) | CUSTOMER | SOURCE_PRESENT | UNKNOWN | no | none |
| D098 | [frappe/erpnext](https://github.com/frappe/erpnext) | COMMERCE | DISCOVERED | UNKNOWN | no | none |
| D099 | [frappe/frappe](https://github.com/frappe/frappe) | COMMERCE | DISCOVERED | UNKNOWN | no | none |
| D100 | [freelawproject/courtlistener](https://github.com/freelawproject/courtlistener) | LEGAL | DISCOVERED | UNKNOWN | no | none |
| D101 | [freelawproject/eyecite](https://github.com/freelawproject/eyecite) | LEGAL | DISCOVERED | UNKNOWN | no | none |
| D102 | [freelawproject/juriscraper](https://github.com/freelawproject/juriscraper) | LEGAL | DISCOVERED | UNKNOWN | no | none |
| D103 | [FreeRTOS/FreeRTOS-Kernel](https://github.com/FreeRTOS/FreeRTOS-Kernel) | EDGE | DISCOVERED | UNKNOWN | no | none |
| D104 | [freqtrade/freqtrade](https://github.com/freqtrade/freqtrade) | TRADING | DISCOVERED | UNKNOWN | no | none |
| D105 | [geoserver/geoserver](https://github.com/geoserver/geoserver) | ESTATE | DISCOVERED | UNKNOWN | no | none |
| D106 | [getlago/lago](https://github.com/getlago/lago) | FINANCE | DISCOVERED | UNKNOWN | no | none |
| D107 | [getsentry/sentry](https://github.com/getsentry/sentry) | OBSERVABILITY | DISCOVERED | UNKNOWN | no | none |
| D108 | [getzep/graphiti](https://github.com/getzep/graphiti) | DURABLE_STATE | SOURCE_PRESENT | UNKNOWN | yes | none |
| D109 | [ggml-org/llama.cpp](https://github.com/ggml-org/llama.cpp) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | no | none |
| D110 | [goharbor/harbor](https://github.com/goharbor/harbor) | PLATFORM_ENGINEERING | SOURCE_PRESENT | UNKNOWN | no | none |
| D111 | [google-deepmind/mujoco](https://github.com/google-deepmind/mujoco) | SIMULATION | DISCOVERED | UNKNOWN | no | none |
| D112 | [google-research/timesfm](https://github.com/google-research/timesfm) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | no | none |
| D113 | [google/cadvisor](https://github.com/google/cadvisor) | OBSERVABILITY | ABSORBING | YES | yes | partial |
| D114 | [google/gvisor](https://github.com/google/gvisor) | CONTAINER | DISCOVERED | UNKNOWN | no | none |
| D115 | [google/timesketch](https://github.com/google/timesketch) | OSINT | DISCOVERED | UNKNOWN | no | none |
| D116 | [grafana/alloy](https://github.com/grafana/alloy) | OBSERVABILITY | DISCOVERED | UNKNOWN | no | none |
| D117 | [grafana/grafana](https://github.com/grafana/grafana) | OBSERVABILITY | SOURCE_PRESENT | UNKNOWN | no | none |
| D118 | [grafana/loki](https://github.com/grafana/loki) | OBSERVABILITY | ABSORBING | YES | yes | partial |
| D119 | [grafana/pyroscope](https://github.com/grafana/pyroscope) | OBSERVABILITY | DISCOVERED | UNKNOWN | no | none |
| D120 | [grafana/tempo](https://github.com/grafana/tempo) | OBSERVABILITY | SOURCE_PRESENT | UNKNOWN | no | none |
| D121 | [graphhopper/graphhopper](https://github.com/graphhopper/graphhopper) | LOGISTICS | DISCOVERED | UNKNOWN | no | none |
| D122 | [growthbook/growthbook](https://github.com/growthbook/growthbook) | COMMERCE | DISCOVERED | UNKNOWN | no | none |
| D123 | [hashicorp/vault](https://github.com/hashicorp/vault) | AUTHORITY | ABSORBING | YES | yes | partial |
| D124 | [hledger/hledger](https://github.com/hledger/hledger) | FINANCE | DISCOVERED | UNKNOWN | no | none |
| D125 | [home-assistant/core](https://github.com/home-assistant/core) | BUILDING | DISCOVERED | UNKNOWN | no | none |
| D126 | [huggingface/smolagents](https://github.com/huggingface/smolagents) | AI_RUNTIME | DISCOVERED | UNKNOWN | no | none |
| D127 | [huggingface/text-generation-inference](https://github.com/huggingface/text-generation-inference) | MODEL_SERVING | DISCOVERED | UNKNOWN | no | none |
| D128 | [huggingface/transformers](https://github.com/huggingface/transformers) | MODEL_SERVING | DISCOVERED | UNKNOWN | no | none |
| D129 | [hummingbot/hummingbot](https://github.com/hummingbot/hummingbot) | TRADING | DISCOVERED | UNKNOWN | no | none |
| D130 | [in-toto/in-toto](https://github.com/in-toto/in-toto) | IDENTITY | DISCOVERED | UNKNOWN | no | none |
| D131 | [Infisical/infisical](https://github.com/Infisical/infisical) | AUTHORITY | SOURCE_PRESENT | UNKNOWN | yes | none |
| D132 | [intelowlproject/IntelOwl](https://github.com/intelowlproject/IntelOwl) | OSINT | DISCOVERED | UNKNOWN | no | none |
| D133 | [istio/istio](https://github.com/istio/istio) | NETWORK | DISCOVERED | UNKNOWN | no | none |
| D134 | [jaegertracing/jaeger](https://github.com/jaegertracing/jaeger) | OBSERVABILITY | DISCOVERED | UNKNOWN | no | none |
| D135 | [k3s-io/k3s](https://github.com/k3s-io/k3s) | PLATFORM_ENGINEERING | SOURCE_PRESENT | UNKNOWN | no | none |
| D136 | [karmada-io/karmada](https://github.com/karmada-io/karmada) | CLOUD | DISCOVERED | UNKNOWN | no | none |
| D137 | [kata-containers/kata-containers](https://github.com/kata-containers/kata-containers) | CONTAINER | DISCOVERED | UNKNOWN | no | none |
| D138 | [kedacore/keda](https://github.com/kedacore/keda) | SERVERLESS | DISCOVERED | UNKNOWN | no | none |
| D139 | [keycloak/keycloak](https://github.com/keycloak/keycloak) | AUTHORITY | SOURCE_PRESENT | UNKNOWN | yes | none |
| D140 | [killbill/killbill](https://github.com/killbill/killbill) | FINANCE | DISCOVERED | UNKNOWN | no | none |
| D141 | [Kilo-Org/kilocode](https://github.com/Kilo-Org/kilocode) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | no | none |
| D142 | [knative/eventing](https://github.com/knative/eventing) | SERVERLESS | DISCOVERED | UNKNOWN | no | none |
| D143 | [knative/serving](https://github.com/knative/serving) | SERVERLESS | DISCOVERED | UNKNOWN | no | none |
| D144 | [kserve/kserve](https://github.com/kserve/kserve) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | no | none |
| D145 | [kubeedge/kubeedge](https://github.com/kubeedge/kubeedge) | CLOUD | DISCOVERED | UNKNOWN | no | none |
| D146 | [kubernetes-sigs/cluster-api](https://github.com/kubernetes-sigs/cluster-api) | CLOUD | DISCOVERED | UNKNOWN | no | none |
| D147 | [kubernetes-sigs/kueue](https://github.com/kubernetes-sigs/kueue) | SERVERLESS | DISCOVERED | UNKNOWN | no | none |
| D148 | [kubernetes/kube-state-metrics](https://github.com/kubernetes/kube-state-metrics) | PLATFORM_ENGINEERING | ABSORBING | YES | yes | partial |
| D149 | [kubevirt/kubevirt](https://github.com/kubevirt/kubevirt) | PLATFORM_ENGINEERING | SOURCE_PRESENT | UNKNOWN | no | none |
| D150 | [LadybirdBrowser/ladybird](https://github.com/LadybirdBrowser/ladybird) | WEB2APP | SOURCE_PRESENT | UNKNOWN | no | none |
| D151 | [langchain-ai/langgraph](https://github.com/langchain-ai/langgraph) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | no | none |
| D152 | [langfuse/langfuse](https://github.com/langfuse/langfuse) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | no | none |
| D153 | [langgenius/dify](https://github.com/langgenius/dify) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | no | none |
| D154 | [laramies/theHarvester](https://github.com/laramies/theHarvester) | OSINT | ABSORBING | YES | yes | partial |
| D155 | [ledger/ledger](https://github.com/ledger/ledger) | FINANCE | DISCOVERED | UNKNOWN | no | none |
| D156 | [letta-ai/letta](https://github.com/letta-ai/letta) | MEMORY | DISCOVERED | UNKNOWN | no | none |
| D157 | [LexPredict/lexpredict-lexnlp](https://github.com/LexPredict/lexpredict-lexnlp) | LEGAL | DISCOVERED | UNKNOWN | no | none |
| D158 | [lf-edge/ekuiper](https://github.com/lf-edge/ekuiper) | EDGE | DISCOVERED | UNKNOWN | no | none |
| D159 | [maplibre/maplibre-gl-js](https://github.com/maplibre/maplibre-gl-js) | ESTATE | DISCOVERED | UNKNOWN | no | none |
| D160 | [maplibre/martin](https://github.com/maplibre/martin) | ESTATE | DISCOVERED | UNKNOWN | no | none |
| D161 | [mastodon/mastodon](https://github.com/mastodon/mastodon) | OTHER | DISCOVERED | UNKNOWN | no | none |
| D162 | [MaterializeInc/materialize](https://github.com/MaterializeInc/materialize) | EVENTING | DISCOVERED | UNKNOWN | no | none |
| D163 | [mautic/mautic](https://github.com/mautic/mautic) | CUSTOMER | DISCOVERED | UNKNOWN | no | none |
| D164 | [medusajs/medusa](https://github.com/medusajs/medusa) | COMMERCE | DISCOVERED | UNKNOWN | no | none |
| D165 | [megadose/holehe](https://github.com/megadose/holehe) | OSINT | ABSORBING | YES | yes | partial |
| D166 | [meilisearch/meilisearch](https://github.com/meilisearch/meilisearch) | DURABLE_STATE | DISCOVERED | UNKNOWN | no | none |
| D167 | [meltano/meltano](https://github.com/meltano/meltano) | PLATFORM_ENGINEERING | DISCOVERED | UNKNOWN | no | none |
| D168 | [mem0ai/mem0](https://github.com/mem0ai/mem0) | MEMORY | DISCOVERED | UNKNOWN | no | none |
| D169 | [metabase/metabase](https://github.com/metabase/metabase) | DURABLE_STATE | SOURCE_PRESENT | UNKNOWN | no | none |
| D170 | [metal3-io/baremetal-operator](https://github.com/metal3-io/baremetal-operator) | CLOUD | DISCOVERED | UNKNOWN | no | none |
| D171 | [micro-ROS/micro_ros_setup](https://github.com/micro-ROS/micro_ros_setup) | ROBOTICS | DISCOVERED | UNKNOWN | no | none |
| D172 | [microsoft/autogen](https://github.com/microsoft/autogen) | AI_RUNTIME | DISCOVERED | UNKNOWN | no | none |
| D173 | [microsoft/playwright](https://github.com/microsoft/playwright) | WEB2APP | SOURCE_PRESENT | UNKNOWN | no | none |
| D174 | [microsoft/presidio](https://github.com/microsoft/presidio) | DOCUMENT | DISCOVERED | UNKNOWN | no | none |
| D175 | [microsoft/semantic-kernel](https://github.com/microsoft/semantic-kernel) | AI_RUNTIME | DISCOVERED | UNKNOWN | no | none |
| D176 | [milvus-io/milvus](https://github.com/milvus-io/milvus) | DURABLE_STATE | DISCOVERED | UNKNOWN | no | none |
| D177 | [minio/minio](https://github.com/minio/minio) | PLATFORM_ENGINEERING | ABSORBING | YES | yes | partial |
| D178 | [MISP/MISP](https://github.com/MISP/MISP) | OSINT | DISCOVERED | UNKNOWN | no | none |
| D179 | [mlflow/mlflow](https://github.com/mlflow/mlflow) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | no | none |
| D180 | [moby/buildkit](https://github.com/moby/buildkit) | PLATFORM_ENGINEERING | SOURCE_PRESENT | UNKNOWN | no | none |
| D181 | [MODSetter/SurfSense](https://github.com/MODSetter/SurfSense) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | no | none |
| D182 | [moj-analytical-services/splink](https://github.com/moj-analytical-services/splink) | OSINT | DISCOVERED | UNKNOWN | no | none |
| D183 | [mojaloop/mojaloop](https://github.com/mojaloop/mojaloop) | BANKING | DISCOVERED | UNKNOWN | no | none |
| D184 | [moveit/moveit2](https://github.com/moveit/moveit2) | ROBOTICS | SOURCE_PRESENT | UNKNOWN | no | none |
| D185 | [mxrch/GHunt](https://github.com/mxrch/GHunt) | OSINT | ABSORBING | YES | yes | partial |
| D186 | [n8n-io/n8n](https://github.com/n8n-io/n8n) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | no | none |
| D187 | [nats-io/nats-server](https://github.com/nats-io/nats-server) | EXECUTION | ABSORBING | YES | yes | partial |
| D188 | [nautechsystems/nautilus_trader](https://github.com/nautechsystems/nautilus_trader) | TRADING | DISCOVERED | UNKNOWN | no | none |
| D189 | [netdata/netdata](https://github.com/netdata/netdata) | OBSERVABILITY | DISCOVERED | UNKNOWN | no | none |
| D190 | [Nixtla/neuralforecast](https://github.com/Nixtla/neuralforecast) | HOSPITALITY | DISCOVERED | UNKNOWN | no | none |
| D191 | [Nixtla/statsforecast](https://github.com/Nixtla/statsforecast) | HOSPITALITY | DISCOVERED | UNKNOWN | no | none |
| D192 | [nocobase/nocobase](https://github.com/nocobase/nocobase) | CUSTOMER | SOURCE_PRESENT | UNKNOWN | no | none |
| D193 | [novu/novu](https://github.com/novu/novu) | HOSPITALITY | DISCOVERED | UNKNOWN | no | none |
| D194 | [OCRmyPDF/OCRmyPDF](https://github.com/OCRmyPDF/OCRmyPDF) | DOCUMENT | DISCOVERED | UNKNOWN | no | none |
| D195 | [odoo/odoo](https://github.com/odoo/odoo) | COMMERCE | DISCOVERED | UNKNOWN | no | none |
| D196 | [ollama/ollama](https://github.com/ollama/ollama) | MODEL_SERVING | DISCOVERED | UNKNOWN | no | none |
| D197 | [open-contracting/kingfisher-collect](https://github.com/open-contracting/kingfisher-collect) | TRADE | DISCOVERED | UNKNOWN | no | none |
| D198 | [open-contracting/kingfisher-process](https://github.com/open-contracting/kingfisher-process) | TRADE | DISCOVERED | UNKNOWN | no | none |
| D199 | [open-policy-agent/opa](https://github.com/open-policy-agent/opa) | AUTHORITY | ABSORBING | YES | yes | partial |
| D200 | [open-rmf/rmf](https://github.com/open-rmf/rmf) | ROBOTICS | SOURCE_PRESENT | UNKNOWN | no | none |
| D201 | [open-telemetry/opentelemetry-collector](https://github.com/open-telemetry/opentelemetry-collector) | OBSERVABILITY | ABSORBING | YES | yes | partial |
| D202 | [open-telemetry/opentelemetry-collector-contrib](https://github.com/open-telemetry/opentelemetry-collector-contrib) | OBSERVABILITY | DISCOVERED | UNKNOWN | no | none |
| D203 | [open-telemetry/opentelemetry-rust](https://github.com/open-telemetry/opentelemetry-rust) | OBSERVABILITY | DISCOVERED | UNKNOWN | no | none |
| D204 | [OpenBankProject/OBP-API](https://github.com/OpenBankProject/OBP-API) | BANKING | DISCOVERED | UNKNOWN | no | none |
| D205 | [openbao/openbao](https://github.com/openbao/openbao) | AUTHORITY | ABSORBING | YES | yes | partial |
| D206 | [openboxes/openboxes](https://github.com/openboxes/openboxes) | TRADE | DISCOVERED | UNKNOWN | no | none |
| D207 | [openclaw/openclaw](https://github.com/openclaw/openclaw) | ROBOTICS | DISCOVERED | UNKNOWN | no | none |
| D208 | [opencost/opencost](https://github.com/opencost/opencost) | FINANCE | DISCOVERED | UNKNOWN | no | none |
| D209 | [OpenCTI-Platform/opencti](https://github.com/OpenCTI-Platform/opencti) | OSINT | DISCOVERED | UNKNOWN | no | none |
| D210 | [OpenEtherCATsociety/SOEM](https://github.com/OpenEtherCATsociety/SOEM) | INDUSTRIAL | DISCOVERED | UNKNOWN | no | none |
| D211 | [openfaas/faas](https://github.com/openfaas/faas) | SERVERLESS | DISCOVERED | UNKNOWN | no | none |
| D212 | [openfga/openfga](https://github.com/openfga/openfga) | AUTHORITY | ABSORBING | YES | yes | partial |
| D213 | [openfisca/openfisca-core](https://github.com/openfisca/openfisca-core) | LEGAL | DISCOVERED | UNKNOWN | no | none |
| D214 | [openhab/openhab-core](https://github.com/openhab/openhab-core) | BUILDING | DISCOVERED | UNKNOWN | no | none |
| D215 | [OpenLMIS/openlmis-ref-distro](https://github.com/OpenLMIS/openlmis-ref-distro) | TRADE | DISCOVERED | UNKNOWN | no | none |
| D216 | [openmeterio/openmeter](https://github.com/openmeterio/openmeter) | FINANCE | DISCOVERED | UNKNOWN | no | none |
| D217 | [OpenNebula/one](https://github.com/OpenNebula/one) | CLOUD | DISCOVERED | UNKNOWN | no | none |
| D218 | [openobserve/openobserve](https://github.com/openobserve/openobserve) | OBSERVABILITY | DISCOVERED | UNKNOWN | no | none |
| D219 | [openremote/openremote](https://github.com/openremote/openremote) | BUILDING | DISCOVERED | UNKNOWN | no | none |
| D220 | [opensanctions/followthemoney](https://github.com/opensanctions/followthemoney) | OSINT | DISCOVERED | UNKNOWN | no | none |
| D221 | [opensanctions/opensanctions](https://github.com/opensanctions/opensanctions) | OSINT | DISCOVERED | UNKNOWN | no | none |
| D222 | [opensanctions/yente](https://github.com/opensanctions/yente) | OSINT | DISCOVERED | UNKNOWN | no | none |
| D223 | [opensearch-project/OpenSearch](https://github.com/opensearch-project/OpenSearch) | DURABLE_STATE | DISCOVERED | UNKNOWN | no | none |
| D224 | [OpenSignLabs/OpenSign](https://github.com/OpenSignLabs/OpenSign) | DOCUMENT | DISCOVERED | UNKNOWN | no | none |
| D225 | [openstack/ironic](https://github.com/openstack/ironic) | CLOUD | DISCOVERED | UNKNOWN | no | none |
| D226 | [openstack/nova](https://github.com/openstack/nova) | PLATFORM_ENGINEERING | SOURCE_PRESENT | UNKNOWN | no | none |
| D227 | [openTCS/opentcs](https://github.com/openTCS/opentcs) | FLEET | DISCOVERED | UNKNOWN | no | none |
| D228 | [opentofu/opentofu](https://github.com/opentofu/opentofu) | PLATFORM_ENGINEERING | SOURCE_PRESENT | UNKNOWN | no | none |
| D229 | [opentripplanner/OpenTripPlanner](https://github.com/opentripplanner/OpenTripPlanner) | LOGISTICS | DISCOVERED | UNKNOWN | no | none |
| D230 | [osm-search/Nominatim](https://github.com/osm-search/Nominatim) | LOGISTICS | DISCOVERED | UNKNOWN | no | none |
| D231 | [owasp-amass/amass](https://github.com/owasp-amass/amass) | OSINT | DISCOVERED | UNKNOWN | no | none |
| D232 | [paperclipai/paperclip](https://github.com/paperclipai/paperclip) | OTHER | DISCOVERED | UNKNOWN | no | none |
| D233 | [pgRouting/pgrouting](https://github.com/pgRouting/pgrouting) | LOGISTICS | DISCOVERED | UNKNOWN | no | none |
| D234 | [pgvector/pgvector](https://github.com/pgvector/pgvector) | DURABLE_STATE | DISCOVERED | UNKNOWN | no | none |
| D235 | [pimcore/pimcore](https://github.com/pimcore/pimcore) | COMMERCE | DISCOVERED | UNKNOWN | no | none |
| D236 | [postgis/postgis](https://github.com/postgis/postgis) | ESTATE | DISCOVERED | UNKNOWN | no | none |
| D237 | [postgres/postgres](https://github.com/postgres/postgres) | DURABLE_STATE | PARTIALLY_ABSORBED | YES | yes | partial |
| D238 | [PostHog/posthog](https://github.com/PostHog/posthog) | DURABLE_STATE | SOURCE_PRESENT | UNKNOWN | no | none |
| D239 | [Project-OSRM/osrm-backend](https://github.com/Project-OSRM/osrm-backend) | LOGISTICS | DISCOVERED | UNKNOWN | no | none |
| D240 | [projectcalico/calico](https://github.com/projectcalico/calico) | NETWORK | DISCOVERED | UNKNOWN | no | none |
| D241 | [projectdiscovery/nuclei](https://github.com/projectdiscovery/nuclei) | OSINT | DISCOVERED | UNKNOWN | no | none |
| D242 | [prometheus/alertmanager](https://github.com/prometheus/alertmanager) | OBSERVABILITY | ABSORBING | YES | yes | partial |
| D243 | [prometheus/blackbox_exporter](https://github.com/prometheus/blackbox_exporter) | OBSERVABILITY | ABSORBING | YES | yes | partial |
| D244 | [prometheus/node_exporter](https://github.com/prometheus/node_exporter) | OBSERVABILITY | ABSORBING | YES | yes | partial |
| D245 | [prometheus/prometheus](https://github.com/prometheus/prometheus) | OBSERVABILITY | SOURCE_PRESENT | UNKNOWN | no | none |
| D246 | [prometheus/pushgateway](https://github.com/prometheus/pushgateway) | OBSERVABILITY | ABSORBING | YES | yes | partial |
| D247 | [PX4/PX4-Autopilot](https://github.com/PX4/PX4-Autopilot) | ROBOTICS | DISCOVERED | UNKNOWN | no | none |
| D248 | [pydantic/pydantic-ai](https://github.com/pydantic/pydantic-ai) | AI_RUNTIME | DISCOVERED | UNKNOWN | no | none |
| D249 | [qdrant/qdrant](https://github.com/qdrant/qdrant) | DURABLE_STATE | SOURCE_PRESENT | UNKNOWN | no | none |
| D250 | [qeeqbox/social-analyzer](https://github.com/qeeqbox/social-analyzer) | OSINT | ABSORBING | YES | yes | partial |
| D251 | [qgis/QGIS](https://github.com/qgis/QGIS) | ESTATE | DISCOVERED | UNKNOWN | no | none |
| D252 | [Qloapps/QloApps](https://github.com/Qloapps/QloApps) | HOSPITALITY | DISCOVERED | UNKNOWN | no | none |
| D253 | [QuantConnect/Lean](https://github.com/QuantConnect/Lean) | TRADING | DISCOVERED | UNKNOWN | no | none |
| D254 | [quickfix/quickfix](https://github.com/quickfix/quickfix) | TRADING | DISCOVERED | UNKNOWN | no | none |
| D255 | [ray-project/ray](https://github.com/ray-project/ray) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | no | none |
| D256 | [RedpandaData/redpanda](https://github.com/RedpandaData/redpanda) | EVENTING | DISCOVERED | UNKNOWN | no | none |
| D257 | [refly-ai/refly](https://github.com/refly-ai/refly) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | no | none |
| D258 | [remotion-dev/remotion](https://github.com/remotion-dev/remotion) | WEB2APP | SOURCE_PRESENT | UNKNOWN | no | none |
| D259 | [restatedev/restate](https://github.com/restatedev/restate) | EXECUTION | ABSORBING | YES | yes | partial |
| D260 | [RisingWaveLabs/risingwave](https://github.com/RisingWaveLabs/risingwave) | EVENTING | DISCOVERED | UNKNOWN | no | none |
| D261 | [rook/rook](https://github.com/rook/rook) | PLATFORM_ENGINEERING | SOURCE_PRESENT | UNKNOWN | no | none |
| D262 | [ros-controls/ros2_control](https://github.com/ros-controls/ros2_control) | ROBOTICS | DISCOVERED | UNKNOWN | no | none |
| D263 | [ros-navigation/navigation2](https://github.com/ros-navigation/navigation2) | ROBOTICS | SOURCE_PRESENT | UNKNOWN | no | none |
| D264 | [ros2/ros2](https://github.com/ros2/ros2) | ROBOTICS | SOURCE_PRESENT | UNKNOWN | no | none |
| D265 | [rudderlabs/rudder-server](https://github.com/rudderlabs/rudder-server) | COMMERCE | DISCOVERED | UNKNOWN | no | none |
| D266 | [run-llama/llama_index](https://github.com/run-llama/llama_index) | MEMORY | DISCOVERED | UNKNOWN | no | none |
| D267 | [saleor/saleor](https://github.com/saleor/saleor) | COMMERCE | DISCOVERED | UNKNOWN | no | none |
| D268 | [searxng/searxng](https://github.com/searxng/searxng) | OSINT | DISCOVERED | UNKNOWN | no | none |
| D269 | [seldonio/seldon-core](https://github.com/seldonio/seldon-core) | MODEL_SERVING | DISCOVERED | UNKNOWN | no | none |
| D270 | [ServiceNow/BrowserGym](https://github.com/ServiceNow/BrowserGym) | WEB2APP | SOURCE_PRESENT | UNKNOWN | no | none |
| D271 | [sherlock-project/sherlock](https://github.com/sherlock-project/sherlock) | OSINT | ABSORBING | YES | yes | partial |
| D272 | [siderolabs/talos](https://github.com/siderolabs/talos) | CLOUD | DISCOVERED | UNKNOWN | no | none |
| D273 | [sigstore/cosign](https://github.com/sigstore/cosign) | IDENTITY | DISCOVERED | UNKNOWN | no | none |
| D274 | [Skyvern-AI/skyvern](https://github.com/Skyvern-AI/skyvern) | WEB2APP | SOURCE_PRESENT | UNKNOWN | no | none |
| D275 | [smicallef/spiderfoot](https://github.com/smicallef/spiderfoot) | OSINT | ABSORBING | YES | yes | partial |
| D276 | [solidusio/solidus](https://github.com/solidusio/solidus) | COMMERCE | DISCOVERED | UNKNOWN | no | none |
| D277 | [soxoj/maigret](https://github.com/soxoj/maigret) | OSINT | ABSORBING | YES | yes | partial |
| D278 | [spiffe/spire](https://github.com/spiffe/spire) | IDENTITY | DISCOVERED | UNKNOWN | no | none |
| D279 | [spree/spree](https://github.com/spree/spree) | COMMERCE | DISCOVERED | UNKNOWN | no | none |
| D280 | [stephane/libmodbus](https://github.com/stephane/libmodbus) | INDUSTRIAL | DISCOVERED | UNKNOWN | no | none |
| D281 | [SteveMacenski/slam_toolbox](https://github.com/SteveMacenski/slam_toolbox) | ROBOTICS | DISCOVERED | UNKNOWN | no | none |
| D282 | [SuiteCRM/SuiteCRM-Core](https://github.com/SuiteCRM/SuiteCRM-Core) | CUSTOMER | SOURCE_PRESENT | UNKNOWN | no | none |
| D283 | [sundowndev/PhoneInfoga](https://github.com/sundowndev/PhoneInfoga) | OSINT | ABSORBING | YES | yes | partial |
| D284 | [supabase/supabase](https://github.com/supabase/supabase) | DURABLE_STATE | SOURCE_PRESENT | UNKNOWN | no | none |
| D285 | [SWE-agent/SWE-agent](https://github.com/SWE-agent/SWE-agent) | AI_RUNTIME | DISCOVERED | UNKNOWN | no | none |
| D286 | [Sylius/Sylius](https://github.com/Sylius/Sylius) | COMMERCE | DISCOVERED | UNKNOWN | no | none |
| D287 | [TauricResearch/TradingAgents](https://github.com/TauricResearch/TradingAgents) | TRADING | DISCOVERED | UNKNOWN | no | none |
| D288 | [tektoncd/pipeline](https://github.com/tektoncd/pipeline) | SERVERLESS | DISCOVERED | UNKNOWN | no | none |
| D289 | [temporalio/temporal](https://github.com/temporalio/temporal) | EXECUTION | SOURCE_PRESENT | UNKNOWN | yes | none |
| D290 | [tesseract-ocr/tesseract](https://github.com/tesseract-ocr/tesseract) | DOCUMENT | DISCOVERED | UNKNOWN | no | none |
| D291 | [thingsboard/thingsboard](https://github.com/thingsboard/thingsboard) | EDGE | DISCOVERED | UNKNOWN | no | none |
| D292 | [ToolJet/ToolJet](https://github.com/ToolJet/ToolJet) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | no | none |
| D293 | [traccar/traccar](https://github.com/traccar/traccar) | FLEET | DISCOVERED | UNKNOWN | no | none |
| D294 | [traefik/traefik](https://github.com/traefik/traefik) | NETWORK | DISCOVERED | UNKNOWN | no | none |
| D295 | [triton-inference-server/server](https://github.com/triton-inference-server/server) | MODEL_SERVING | DISCOVERED | UNKNOWN | no | none |
| D296 | [twentyhq/twenty](https://github.com/twentyhq/twenty) | CUSTOMER | DISCOVERED | UNKNOWN | no | none |
| D297 | [typesense/typesense](https://github.com/typesense/typesense) | DURABLE_STATE | DISCOVERED | UNKNOWN | no | none |
| D298 | [uber/h3](https://github.com/uber/h3) | LOGISTICS | DISCOVERED | UNKNOWN | no | none |
| D299 | [unclecode/crawl4ai](https://github.com/unclecode/crawl4ai) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | no | none |
| D300 | [unit8co/darts](https://github.com/unit8co/darts) | HOSPITALITY | DISCOVERED | UNKNOWN | no | none |
| D301 | [Unleash/unleash](https://github.com/Unleash/unleash) | COMMERCE | DISCOVERED | UNKNOWN | no | none |
| D302 | [valhalla/valhalla](https://github.com/valhalla/valhalla) | LOGISTICS | DISCOVERED | UNKNOWN | no | none |
| D303 | [valkey-io/valkey](https://github.com/valkey-io/valkey) | DURABLE_STATE | DISCOVERED | UNKNOWN | no | none |
| D304 | [vendure-ecommerce/vendure](https://github.com/vendure-ecommerce/vendure) | COMMERCE | DISCOVERED | UNKNOWN | no | none |
| D305 | [vercel-labs/agent-browser](https://github.com/vercel-labs/agent-browser) | OTHER | DISCOVERED | UNKNOWN | no | none |
| D306 | [vitessio/vitess](https://github.com/vitessio/vitess) | DURABLE_STATE | DISCOVERED | UNKNOWN | no | none |
| D307 | [vllm-project/vllm](https://github.com/vllm-project/vllm) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | no | none |
| D308 | [vnpy/vnpy](https://github.com/vnpy/vnpy) | TRADING | DISCOVERED | UNKNOWN | no | none |
| D309 | [volcano-sh/volcano](https://github.com/volcano-sh/volcano) | SERVERLESS | DISCOVERED | UNKNOWN | no | none |
| D310 | [VROOM-Project/vroom](https://github.com/VROOM-Project/vroom) | LOGISTICS | DISCOVERED | UNKNOWN | no | none |
| D311 | [wasmCloud/wasmCloud](https://github.com/wasmCloud/wasmCloud) | CONTAINER | DISCOVERED | UNKNOWN | no | none |
| D312 | [weaviate/weaviate](https://github.com/weaviate/weaviate) | DURABLE_STATE | DISCOVERED | UNKNOWN | no | none |
| D313 | [XKNX/xknx](https://github.com/XKNX/xknx) | BUILDING | DISCOVERED | UNKNOWN | no | none |
| D314 | [yaojingang/GEOFlow](https://github.com/yaojingang/GEOFlow) | LOGISTICS | DISCOVERED | UNKNOWN | no | none |
| D315 | [Z4nzu/hackingtool](https://github.com/Z4nzu/hackingtool) | OSINT | ABSORBING | YES | yes | partial |
| D316 | [zed-industries/zed](https://github.com/zed-industries/zed) | DEVELOPER_TOOLING | DISCOVERED | UNKNOWN | no | none |
| D317 | [zephyrproject-rtos/zephyr](https://github.com/zephyrproject-rtos/zephyr) | EDGE | DISCOVERED | UNKNOWN | no | none |
| D318 | [open62541/open62541](https://github.com/open62541/open62541) | INDUSTRIAL | DISCOVERED | UNKNOWN | no | none |
| D319 | [facebookresearch/vjepa2](https://github.com/facebookresearch/vjepa2) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | yes | none |
| D320 | [danijar/dreamerv3](https://github.com/danijar/dreamerv3) | AI_RUNTIME | SOURCE_PRESENT | UNKNOWN | yes | none |
| D321 | [gazebosim/gz-sim](https://github.com/gazebosim/gz-sim) | SIMULATION | DISCOVERED | UNKNOWN | no | none |
| D322 | [apache/fineract](https://github.com/apache/fineract) | BANKING | DISCOVERED | UNKNOWN | no | none |
| D323 | [element-hq/synapse](https://github.com/element-hq/synapse) | OTHER | DISCOVERED | UNKNOWN | no | none |
