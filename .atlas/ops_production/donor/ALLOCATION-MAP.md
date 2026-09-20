# Donor Allocation Map

> Generated governance snapshot for the 2026-09-19 refoundation wave. Source of truth remains `tools/refoundation/donor-corpus.yaml` plus `tools/system-atlas/fleet/ops-registry.yaml`. Do not hand-edit individual assignments here without changing those sources.

This map closes the donor ownership question for the current corpus: **323/323 donors allocated, 0 unallocated**. Broad technical families default to Development Cells; admission into Chronica is explicit. Legacy `Host-Ops-PMS` is inactive so hospitality remains one universe under `bnbOps`.

## Allocation totals

| Primary absorber | Donors |
|---|---:|
| `llgtrn/BankingOps` | 3 |
| `llgtrn/bnbOps` | 7 |
| `llgtrn/Chronica` | 14 |
| `llgtrn/CustomerOps` | 11 |
| `llgtrn/ECOps` | 15 |
| `llgtrn/EdgeOps` | 21 |
| `llgtrn/EstateOps` | 5 |
| `llgtrn/FinOps` | 7 |
| `llgtrn/FleetOps` | 10 |
| `llgtrn/HelpdeskOps` | 1 |
| `llgtrn/IndustrialOps` | 7 |
| `llgtrn/LegalOps` | 7 |
| `llgtrn/OSINTOps` | 30 |
| `llgtrn/PlatformOps` | 130 |
| `llgtrn/RoboticsOps` | 14 |
| `llgtrn/SimulationOps` | 3 |
| `llgtrn/StayChatbotOps` | 3 |
| `llgtrn/TradeOps` | 11 |
| `llgtrn/TradingOps` | 8 |
| `llgtrn/TwinOps` | 5 |
| `llgtrn/W2AOps` | 11 |

## Chronica-owned donors

Chronica is not a catch-all technical target. The 14 Chronica-owned donors are split into 8 foundation references/absorption targets, 2 organism references, and 4 accounting-only comparison references.

| ID | Repo | Class | Status |
|---|---|---|---|
| D007 | `apache/age` | CHRONICA_FOUNDATION | ABSORBING |
| D024 | `authzed/spicedb` | CHRONICA_FOUNDATION | ABSORBING |
| D108 | `getzep/graphiti` | CHRONICA_FOUNDATION | SOURCE_PRESENT |
| D123 | `hashicorp/vault` | REFERENCE_ONLY | REFERENCE_ONLY |
| D131 | `Infisical/infisical` | REFERENCE_ONLY | REFERENCE_ONLY |
| D139 | `keycloak/keycloak` | CHRONICA_FOUNDATION | SOURCE_PRESENT |
| D199 | `open-policy-agent/opa` | CHRONICA_FOUNDATION | ABSORBING |
| D205 | `openbao/openbao` | CHRONICA_FOUNDATION | ABSORBING |
| D212 | `openfga/openfga` | REFERENCE_ONLY | REFERENCE_ONLY |
| D237 | `postgres/postgres` | CHRONICA_FOUNDATION | ABSORBING |
| D259 | `restatedev/restate` | CHRONICA_FOUNDATION | ABSORBING |
| D289 | `temporalio/temporal` | REFERENCE_ONLY | REFERENCE_ONLY |
| D319 | `facebookresearch/vjepa2` | CHRONICA_ORGANISM | DISCOVERED |
| D320 | `danijar/dreamerv3` | CHRONICA_ORGANISM | DISCOVERED |

## Full closed-world allocation (323)

| ID | Donor | Primary absorber | Class | Family | Status |
|---|---|---|---|---|---|
| D001 | `activepieces/activepieces` | `llgtrn/CustomerOps` | DEVELOPMENT_CELL | CUSTOMER | SOURCE_PRESENT |
| D002 | `adbar/trafilatura` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | DISCOVERED |
| D003 | `AIDC-AI/Pixelle-Video` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | SOURCE_PRESENT |
| D004 | `akeneo/pim-community-dev` | `llgtrn/ECOps` | DEVELOPMENT_CELL | COMMERCE | DISCOVERED |
| D005 | `alephdata/aleph` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | DISCOVERED |
| D006 | `All-Hands-AI/OpenHands` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | DISCOVERED |
| D007 | `apache/age` | `llgtrn/Chronica` | CHRONICA_FOUNDATION | DURABLE_STATE | ABSORBING |
| D008 | `apache/airflow` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | SERVERLESS | DISCOVERED |
| D009 | `apache/camel` | `llgtrn/TradeOps` | DEVELOPMENT_CELL | TRADE | DISCOVERED |
| D010 | `apache/cloudstack` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | CLOUD | DISCOVERED |
| D011 | `apache/flink` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | EXECUTION | SOURCE_PRESENT |
| D012 | `apache/hudi` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DURABLE_STATE | DISCOVERED |
| D013 | `apache/iceberg` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DURABLE_STATE | DISCOVERED |
| D014 | `apache/kafka` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | EXECUTION | ABSORBING |
| D015 | `apache/pulsar` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | EVENTING | DISCOVERED |
| D016 | `apache/tika` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | DISCOVERED |
| D017 | `apify/crawlee` | `llgtrn/W2AOps` | DEVELOPMENT_CELL | WEB2APP | SOURCE_PRESENT |
| D018 | `appsmithorg/appsmith` | `llgtrn/CustomerOps` | DEVELOPMENT_CELL | CUSTOMER | SOURCE_PRESENT |
| D019 | `aquasecurity/tracee` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | SECURITY | DISCOVERED |
| D020 | `aquasecurity/trivy` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | SECURITY | DISCOVERED |
| D021 | `ArduPilot/ardupilot` | `llgtrn/RoboticsOps` | DEVELOPMENT_CELL | ROBOTICS | DISCOVERED |
| D022 | `argoproj/argo-cd` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | SOURCE_PRESENT |
| D023 | `argoproj/argo-workflows` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | SERVERLESS | DISCOVERED |
| D024 | `authzed/spicedb` | `llgtrn/Chronica` | CHRONICA_FOUNDATION | AUTHORITY | ABSORBING |
| D025 | `AutowareFoundation/autoware_universe` | `llgtrn/RoboticsOps` | DEVELOPMENT_CELL | ROBOTICS | DISCOVERED |
| D026 | `awslabs/gluonts` | `llgtrn/bnbOps` | DEVELOPMENT_CELL | HOSPITALITY | DISCOVERED |
| D027 | `backstage/backstage` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | DISCOVERED |
| D028 | `bacnet-stack/bacnet-stack` | `llgtrn/TwinOps` | DEVELOPMENT_CELL | BUILDING | DISCOVERED |
| D029 | `bagisto/bagisto` | `llgtrn/ECOps` | DEVELOPMENT_CELL | COMMERCE | DISCOVERED |
| D030 | `baserow/baserow` | `llgtrn/CustomerOps` | DEVELOPMENT_CELL | CUSTOMER | SOURCE_PRESENT |
| D031 | `beancount/beancount` | `llgtrn/FinOps` | DEVELOPMENT_CELL | FINANCE | DISCOVERED |
| D032 | `bentoml/BentoML` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | MODEL_SERVING | DISCOVERED |
| D033 | `browser-use/browser-use` | `llgtrn/W2AOps` | DEVELOPMENT_CELL | WEB2APP | SOURCE_PRESENT |
| D034 | `browserbase/stagehand` | `llgtrn/W2AOps` | DEVELOPMENT_CELL | WEB2APP | SOURCE_PRESENT |
| D035 | `bytecodealliance/wasmtime` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | CONTAINER | DISCOVERED |
| D036 | `bytedance/deer-flow` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | SOURCE_PRESENT |
| D037 | `calcom/cal.com` | `llgtrn/StayChatbotOps` | DEVELOPMENT_CELL | HOSPITALITY | DISCOVERED |
| D038 | `CANopenNode/CANopenNode` | `llgtrn/IndustrialOps` | DEVELOPMENT_CELL | INDUSTRIAL | DISCOVERED |
| D039 | `ceph/ceph` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | SOURCE_PRESENT |
| D040 | `cert-manager/cert-manager` | `llgtrn/TradeOps` | DEVELOPMENT_CELL | IDENTITY | DISCOVERED |
| D041 | `chatwoot/chatwoot` | `llgtrn/HelpdeskOps` | DEVELOPMENT_CELL | CUSTOMER | DISCOVERED |
| D042 | `cilium/cilium` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | ABSORBING |
| D043 | `cilium/tetragon` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | NETWORK | DISCOVERED |
| D044 | `ClickHouse/ClickHouse` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DURABLE_STATE | SOURCE_PRESENT |
| D045 | `cloud-hypervisor/cloud-hypervisor` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | CONTAINER | DISCOVERED |
| D046 | `cloudnative-pg/cloudnative-pg` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DURABLE_STATE | DISCOVERED |
| D047 | `comfyanonymous/ComfyUI` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | SOURCE_PRESENT |
| D048 | `containerd/containerd` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | ABSORBING |
| D049 | `containers/podman-desktop` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | SOURCE_PRESENT |
| D050 | `coollabsio/coolify` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | SOURCE_PRESENT |
| D051 | `coredns/coredns` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | ABSORBING |
| D052 | `coroot/coroot` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | DISCOVERED |
| D053 | `cortezaproject/corteza` | `llgtrn/CustomerOps` | DEVELOPMENT_CELL | CUSTOMER | SOURCE_PRESENT |
| D054 | `crewAIInc/crewAI` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | DISCOVERED |
| D055 | `crossplane/crossplane` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | SOURCE_PRESENT |
| D056 | `D4Vinci/Scrapling` | `llgtrn/W2AOps` | DEVELOPMENT_CELL | WEB2APP | DISCOVERED |
| D057 | `dagger/dagger` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | MODEL_SERVING | DISCOVERED |
| D058 | `dagster-io/dagster` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | SERVERLESS | DISCOVERED |
| D059 | `dapr/dapr` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | DISCOVERED |
| D060 | `daytonaio/daytona` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | SOURCE_PRESENT |
| D061 | `dbt-labs/dbt-core` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DURABLE_STATE | DISCOVERED |
| D062 | `debezium/debezium` | `llgtrn/TradeOps` | DEVELOPMENT_CELL | TRADE | DISCOVERED |
| D063 | `decolua/9router` | `llgtrn/W2AOps` | DEVELOPMENT_CELL | WEB2APP | DISCOVERED |
| D064 | `deepset-ai/haystack` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | MEMORY | DISCOVERED |
| D065 | `delta-io/delta` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DURABLE_STATE | DISCOVERED |
| D066 | `dgtlmoon/changedetection.io` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | DISCOVERED |
| D067 | `digital-druid/hoteldruid` | `llgtrn/bnbOps` | DEVELOPMENT_CELL | HOSPITALITY | DISCOVERED |
| D068 | `directus/directus` | `llgtrn/CustomerOps` | DEVELOPMENT_CELL | CUSTOMER | SOURCE_PRESENT |
| D069 | `docassemble/docassemble` | `llgtrn/LegalOps` | DEVELOPMENT_CELL | LEGAL | DISCOVERED |
| D070 | `documenso/documenso` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | DOCUMENT | DISCOVERED |
| D071 | `duckdb/duckdb` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DURABLE_STATE | DISCOVERED |
| D072 | `e2b-dev/E2B` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | MODEL_SERVING | DISCOVERED |
| D073 | `eclipse-cyclonedds/cyclonedds` | `llgtrn/RoboticsOps` | DEVELOPMENT_CELL | ROBOTICS | DISCOVERED |
| D074 | `eclipse-ditto/ditto` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | EDGE | DISCOVERED |
| D075 | `eclipse-hono/hono` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | EDGE | DISCOVERED |
| D076 | `eclipse-kanto/kanto` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | EDGE | DISCOVERED |
| D077 | `eclipse-mosquitto/mosquitto` | `llgtrn/IndustrialOps` | DEVELOPMENT_CELL | INDUSTRIAL | DISCOVERED |
| D078 | `eclipse-sumo/sumo` | `llgtrn/SimulationOps` | DEVELOPMENT_CELL | SIMULATION | DISCOVERED |
| D079 | `eclipse-tahu/tahu` | `llgtrn/IndustrialOps` | DEVELOPMENT_CELL | INDUSTRIAL | DISCOVERED |
| D080 | `eclipse-zenoh/zenoh` | `llgtrn/RoboticsOps` | DEVELOPMENT_CELL | ROBOTICS | DISCOVERED |
| D081 | `edgexfoundry/edgex-go` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | EDGE | DISCOVERED |
| D082 | `elceef/dnstwist` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | ABSORBING |
| D083 | `emqx/emqx` | `llgtrn/IndustrialOps` | DEVELOPMENT_CELL | INDUSTRIAL | DISCOVERED |
| D084 | `envoyproxy/envoy` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | ABSORBING |
| D085 | `envoyproxy/gateway` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | NETWORK | DISCOVERED |
| D086 | `eProsima/Fast-DDS` | `llgtrn/RoboticsOps` | DEVELOPMENT_CELL | ROBOTICS | DISCOVERED |
| D087 | `esig/dss` | `llgtrn/LegalOps` | DEVELOPMENT_CELL | LEGAL | DISCOVERED |
| D088 | `espocrm/espocrm` | `llgtrn/CustomerOps` | DEVELOPMENT_CELL | CUSTOMER | SOURCE_PRESENT |
| D089 | `facebook/prophet` | `llgtrn/bnbOps` | DEVELOPMENT_CELL | HOSPITALITY | DISCOVERED |
| D090 | `falcosecurity/falco` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | AUTHORITY | SOURCE_PRESENT |
| D091 | `fermyon/spin` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | CONTAINER | DISCOVERED |
| D092 | `Fincept-Corporation/FinceptTerminal` | `llgtrn/TradingOps` | DEVELOPMENT_CELL | TRADING | DISCOVERED |
| D093 | `firecracker-microvm/firecracker` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | ABSORBING |
| D094 | `flatcar/Flatcar` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | CLOUD | DISCOVERED |
| D095 | `fluent/fluent-bit` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | DISCOVERED |
| D096 | `formbricks/formbricks` | `llgtrn/StayChatbotOps` | DEVELOPMENT_CELL | HOSPITALITY | DISCOVERED |
| D097 | `frappe/crm` | `llgtrn/CustomerOps` | DEVELOPMENT_CELL | CUSTOMER | SOURCE_PRESENT |
| D098 | `frappe/erpnext` | `llgtrn/ECOps` | DEVELOPMENT_CELL | COMMERCE | DISCOVERED |
| D099 | `frappe/frappe` | `llgtrn/ECOps` | DEVELOPMENT_CELL | COMMERCE | DISCOVERED |
| D100 | `freelawproject/courtlistener` | `llgtrn/LegalOps` | DEVELOPMENT_CELL | LEGAL | DISCOVERED |
| D101 | `freelawproject/eyecite` | `llgtrn/LegalOps` | DEVELOPMENT_CELL | LEGAL | DISCOVERED |
| D102 | `freelawproject/juriscraper` | `llgtrn/LegalOps` | DEVELOPMENT_CELL | LEGAL | DISCOVERED |
| D103 | `FreeRTOS/FreeRTOS-Kernel` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | EDGE | DISCOVERED |
| D104 | `freqtrade/freqtrade` | `llgtrn/TradingOps` | DEVELOPMENT_CELL | TRADING | DISCOVERED |
| D105 | `geoserver/geoserver` | `llgtrn/EstateOps` | DEVELOPMENT_CELL | ESTATE | DISCOVERED |
| D106 | `getlago/lago` | `llgtrn/FinOps` | DEVELOPMENT_CELL | FINANCE | DISCOVERED |
| D107 | `getsentry/sentry` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | DISCOVERED |
| D108 | `getzep/graphiti` | `llgtrn/Chronica` | CHRONICA_FOUNDATION | DURABLE_STATE | SOURCE_PRESENT |
| D109 | `ggml-org/llama.cpp` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | SOURCE_PRESENT |
| D110 | `goharbor/harbor` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | SOURCE_PRESENT |
| D111 | `google-deepmind/mujoco` | `llgtrn/SimulationOps` | DEVELOPMENT_CELL | SIMULATION | DISCOVERED |
| D112 | `google-research/timesfm` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | SOURCE_PRESENT |
| D113 | `google/cadvisor` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | ABSORBING |
| D114 | `google/gvisor` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | CONTAINER | DISCOVERED |
| D115 | `google/timesketch` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | DISCOVERED |
| D116 | `grafana/alloy` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | DISCOVERED |
| D117 | `grafana/grafana` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | SOURCE_PRESENT |
| D118 | `grafana/loki` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | ABSORBING |
| D119 | `grafana/pyroscope` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | DISCOVERED |
| D120 | `grafana/tempo` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | SOURCE_PRESENT |
| D121 | `graphhopper/graphhopper` | `llgtrn/FleetOps` | DEVELOPMENT_CELL | LOGISTICS | DISCOVERED |
| D122 | `growthbook/growthbook` | `llgtrn/ECOps` | DEVELOPMENT_CELL | COMMERCE | DISCOVERED |
| D123 | `hashicorp/vault` | `llgtrn/Chronica` | REFERENCE_ONLY | AUTHORITY | REFERENCE_ONLY |
| D124 | `hledger/hledger` | `llgtrn/FinOps` | DEVELOPMENT_CELL | FINANCE | DISCOVERED |
| D125 | `home-assistant/core` | `llgtrn/TwinOps` | DEVELOPMENT_CELL | BUILDING | DISCOVERED |
| D126 | `huggingface/smolagents` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | DISCOVERED |
| D127 | `huggingface/text-generation-inference` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | MODEL_SERVING | DISCOVERED |
| D128 | `huggingface/transformers` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | MODEL_SERVING | DISCOVERED |
| D129 | `hummingbot/hummingbot` | `llgtrn/TradingOps` | DEVELOPMENT_CELL | TRADING | DISCOVERED |
| D130 | `in-toto/in-toto` | `llgtrn/TradeOps` | DEVELOPMENT_CELL | IDENTITY | DISCOVERED |
| D131 | `Infisical/infisical` | `llgtrn/Chronica` | REFERENCE_ONLY | AUTHORITY | REFERENCE_ONLY |
| D132 | `intelowlproject/IntelOwl` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | DISCOVERED |
| D133 | `istio/istio` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | NETWORK | DISCOVERED |
| D134 | `jaegertracing/jaeger` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | DISCOVERED |
| D135 | `k3s-io/k3s` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | SOURCE_PRESENT |
| D136 | `karmada-io/karmada` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | CLOUD | DISCOVERED |
| D137 | `kata-containers/kata-containers` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | CONTAINER | DISCOVERED |
| D138 | `kedacore/keda` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | SERVERLESS | DISCOVERED |
| D139 | `keycloak/keycloak` | `llgtrn/Chronica` | CHRONICA_FOUNDATION | AUTHORITY | SOURCE_PRESENT |
| D140 | `killbill/killbill` | `llgtrn/FinOps` | DEVELOPMENT_CELL | FINANCE | DISCOVERED |
| D141 | `Kilo-Org/kilocode` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | SOURCE_PRESENT |
| D142 | `knative/eventing` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | SERVERLESS | DISCOVERED |
| D143 | `knative/serving` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | SERVERLESS | DISCOVERED |
| D144 | `kserve/kserve` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | SOURCE_PRESENT |
| D145 | `kubeedge/kubeedge` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | CLOUD | DISCOVERED |
| D146 | `kubernetes-sigs/cluster-api` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | CLOUD | DISCOVERED |
| D147 | `kubernetes-sigs/kueue` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | SERVERLESS | DISCOVERED |
| D148 | `kubernetes/kube-state-metrics` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | ABSORBING |
| D149 | `kubevirt/kubevirt` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | SOURCE_PRESENT |
| D150 | `LadybirdBrowser/ladybird` | `llgtrn/W2AOps` | DEVELOPMENT_CELL | WEB2APP | SOURCE_PRESENT |
| D151 | `langchain-ai/langgraph` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | SOURCE_PRESENT |
| D152 | `langfuse/langfuse` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | SOURCE_PRESENT |
| D153 | `langgenius/dify` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | SOURCE_PRESENT |
| D154 | `laramies/theHarvester` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | ABSORBING |
| D155 | `ledger/ledger` | `llgtrn/FinOps` | DEVELOPMENT_CELL | FINANCE | DISCOVERED |
| D156 | `letta-ai/letta` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | MEMORY | DISCOVERED |
| D157 | `LexPredict/lexpredict-lexnlp` | `llgtrn/LegalOps` | DEVELOPMENT_CELL | LEGAL | DISCOVERED |
| D158 | `lf-edge/ekuiper` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | EDGE | DISCOVERED |
| D159 | `maplibre/maplibre-gl-js` | `llgtrn/EstateOps` | DEVELOPMENT_CELL | ESTATE | DISCOVERED |
| D160 | `maplibre/martin` | `llgtrn/EstateOps` | DEVELOPMENT_CELL | ESTATE | DISCOVERED |
| D161 | `mastodon/mastodon` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OTHER | DISCOVERED |
| D162 | `MaterializeInc/materialize` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | EVENTING | DISCOVERED |
| D163 | `mautic/mautic` | `llgtrn/CustomerOps` | DEVELOPMENT_CELL | CUSTOMER | DISCOVERED |
| D164 | `medusajs/medusa` | `llgtrn/ECOps` | DEVELOPMENT_CELL | COMMERCE | DISCOVERED |
| D165 | `megadose/holehe` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | ABSORBING |
| D166 | `meilisearch/meilisearch` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DURABLE_STATE | DISCOVERED |
| D167 | `meltano/meltano` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | DISCOVERED |
| D168 | `mem0ai/mem0` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | MEMORY | DISCOVERED |
| D169 | `metabase/metabase` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DURABLE_STATE | SOURCE_PRESENT |
| D170 | `metal3-io/baremetal-operator` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | CLOUD | DISCOVERED |
| D171 | `micro-ROS/micro_ros_setup` | `llgtrn/RoboticsOps` | DEVELOPMENT_CELL | ROBOTICS | DISCOVERED |
| D172 | `microsoft/autogen` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | DISCOVERED |
| D173 | `microsoft/playwright` | `llgtrn/W2AOps` | DEVELOPMENT_CELL | WEB2APP | SOURCE_PRESENT |
| D174 | `microsoft/presidio` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | DOCUMENT | DISCOVERED |
| D175 | `microsoft/semantic-kernel` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | DISCOVERED |
| D176 | `milvus-io/milvus` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DURABLE_STATE | DISCOVERED |
| D177 | `minio/minio` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | ABSORBING |
| D178 | `MISP/MISP` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | DISCOVERED |
| D179 | `mlflow/mlflow` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | SOURCE_PRESENT |
| D180 | `moby/buildkit` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | SOURCE_PRESENT |
| D181 | `MODSetter/SurfSense` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | SOURCE_PRESENT |
| D182 | `moj-analytical-services/splink` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | DISCOVERED |
| D183 | `mojaloop/mojaloop` | `llgtrn/BankingOps` | DEVELOPMENT_CELL | BANKING | DISCOVERED |
| D184 | `moveit/moveit2` | `llgtrn/RoboticsOps` | DEVELOPMENT_CELL | ROBOTICS | SOURCE_PRESENT |
| D185 | `mxrch/GHunt` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | ABSORBING |
| D186 | `n8n-io/n8n` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | SOURCE_PRESENT |
| D187 | `nats-io/nats-server` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | EXECUTION | ABSORBING |
| D188 | `nautechsystems/nautilus_trader` | `llgtrn/TradingOps` | DEVELOPMENT_CELL | TRADING | DISCOVERED |
| D189 | `netdata/netdata` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | DISCOVERED |
| D190 | `Nixtla/neuralforecast` | `llgtrn/bnbOps` | DEVELOPMENT_CELL | HOSPITALITY | DISCOVERED |
| D191 | `Nixtla/statsforecast` | `llgtrn/bnbOps` | DEVELOPMENT_CELL | HOSPITALITY | DISCOVERED |
| D192 | `nocobase/nocobase` | `llgtrn/CustomerOps` | DEVELOPMENT_CELL | CUSTOMER | SOURCE_PRESENT |
| D193 | `novu/novu` | `llgtrn/StayChatbotOps` | DEVELOPMENT_CELL | HOSPITALITY | DISCOVERED |
| D194 | `OCRmyPDF/OCRmyPDF` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | DOCUMENT | DISCOVERED |
| D195 | `odoo/odoo` | `llgtrn/ECOps` | DEVELOPMENT_CELL | COMMERCE | DISCOVERED |
| D196 | `ollama/ollama` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | MODEL_SERVING | DISCOVERED |
| D197 | `open-contracting/kingfisher-collect` | `llgtrn/TradeOps` | DEVELOPMENT_CELL | TRADE | DISCOVERED |
| D198 | `open-contracting/kingfisher-process` | `llgtrn/TradeOps` | DEVELOPMENT_CELL | TRADE | DISCOVERED |
| D199 | `open-policy-agent/opa` | `llgtrn/Chronica` | CHRONICA_FOUNDATION | AUTHORITY | ABSORBING |
| D200 | `open-rmf/rmf` | `llgtrn/RoboticsOps` | DEVELOPMENT_CELL | ROBOTICS | SOURCE_PRESENT |
| D201 | `open-telemetry/opentelemetry-collector` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | ABSORBING |
| D202 | `open-telemetry/opentelemetry-collector-contrib` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | DISCOVERED |
| D203 | `open-telemetry/opentelemetry-rust` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | DISCOVERED |
| D204 | `OpenBankProject/OBP-API` | `llgtrn/BankingOps` | DEVELOPMENT_CELL | BANKING | DISCOVERED |
| D205 | `openbao/openbao` | `llgtrn/Chronica` | CHRONICA_FOUNDATION | AUTHORITY | ABSORBING |
| D206 | `openboxes/openboxes` | `llgtrn/TradeOps` | DEVELOPMENT_CELL | TRADE | DISCOVERED |
| D207 | `openclaw/openclaw` | `llgtrn/RoboticsOps` | DEVELOPMENT_CELL | ROBOTICS | DISCOVERED |
| D208 | `opencost/opencost` | `llgtrn/FinOps` | DEVELOPMENT_CELL | FINANCE | DISCOVERED |
| D209 | `OpenCTI-Platform/opencti` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | DISCOVERED |
| D210 | `OpenEtherCATsociety/SOEM` | `llgtrn/IndustrialOps` | DEVELOPMENT_CELL | INDUSTRIAL | DISCOVERED |
| D211 | `openfaas/faas` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | SERVERLESS | DISCOVERED |
| D212 | `openfga/openfga` | `llgtrn/Chronica` | REFERENCE_ONLY | AUTHORITY | REFERENCE_ONLY |
| D213 | `openfisca/openfisca-core` | `llgtrn/LegalOps` | DEVELOPMENT_CELL | LEGAL | DISCOVERED |
| D214 | `openhab/openhab-core` | `llgtrn/TwinOps` | DEVELOPMENT_CELL | BUILDING | DISCOVERED |
| D215 | `OpenLMIS/openlmis-ref-distro` | `llgtrn/TradeOps` | DEVELOPMENT_CELL | TRADE | DISCOVERED |
| D216 | `openmeterio/openmeter` | `llgtrn/FinOps` | DEVELOPMENT_CELL | FINANCE | DISCOVERED |
| D217 | `OpenNebula/one` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | CLOUD | DISCOVERED |
| D218 | `openobserve/openobserve` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | DISCOVERED |
| D219 | `openremote/openremote` | `llgtrn/TwinOps` | DEVELOPMENT_CELL | BUILDING | DISCOVERED |
| D220 | `opensanctions/followthemoney` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | DISCOVERED |
| D221 | `opensanctions/opensanctions` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | DISCOVERED |
| D222 | `opensanctions/yente` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | DISCOVERED |
| D223 | `opensearch-project/OpenSearch` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DURABLE_STATE | DISCOVERED |
| D224 | `OpenSignLabs/OpenSign` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | DOCUMENT | DISCOVERED |
| D225 | `openstack/ironic` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | CLOUD | DISCOVERED |
| D226 | `openstack/nova` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | SOURCE_PRESENT |
| D227 | `openTCS/opentcs` | `llgtrn/FleetOps` | DEVELOPMENT_CELL | FLEET | DISCOVERED |
| D228 | `opentofu/opentofu` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | SOURCE_PRESENT |
| D229 | `opentripplanner/OpenTripPlanner` | `llgtrn/FleetOps` | DEVELOPMENT_CELL | LOGISTICS | DISCOVERED |
| D230 | `osm-search/Nominatim` | `llgtrn/FleetOps` | DEVELOPMENT_CELL | LOGISTICS | DISCOVERED |
| D231 | `owasp-amass/amass` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | DISCOVERED |
| D232 | `paperclipai/paperclip` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OTHER | DISCOVERED |
| D233 | `pgRouting/pgrouting` | `llgtrn/FleetOps` | DEVELOPMENT_CELL | LOGISTICS | DISCOVERED |
| D234 | `pgvector/pgvector` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DURABLE_STATE | DISCOVERED |
| D235 | `pimcore/pimcore` | `llgtrn/ECOps` | DEVELOPMENT_CELL | COMMERCE | DISCOVERED |
| D236 | `postgis/postgis` | `llgtrn/EstateOps` | DEVELOPMENT_CELL | ESTATE | DISCOVERED |
| D237 | `postgres/postgres` | `llgtrn/Chronica` | CHRONICA_FOUNDATION | DURABLE_STATE | ABSORBING |
| D238 | `PostHog/posthog` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DURABLE_STATE | SOURCE_PRESENT |
| D239 | `Project-OSRM/osrm-backend` | `llgtrn/FleetOps` | DEVELOPMENT_CELL | LOGISTICS | DISCOVERED |
| D240 | `projectcalico/calico` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | NETWORK | DISCOVERED |
| D241 | `projectdiscovery/nuclei` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | DISCOVERED |
| D242 | `prometheus/alertmanager` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | ABSORBING |
| D243 | `prometheus/blackbox_exporter` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | ABSORBING |
| D244 | `prometheus/node_exporter` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | ABSORBING |
| D245 | `prometheus/prometheus` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | SOURCE_PRESENT |
| D246 | `prometheus/pushgateway` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OBSERVABILITY | ABSORBING |
| D247 | `PX4/PX4-Autopilot` | `llgtrn/RoboticsOps` | DEVELOPMENT_CELL | ROBOTICS | DISCOVERED |
| D248 | `pydantic/pydantic-ai` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | DISCOVERED |
| D249 | `qdrant/qdrant` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DURABLE_STATE | SOURCE_PRESENT |
| D250 | `qeeqbox/social-analyzer` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | ABSORBING |
| D251 | `qgis/QGIS` | `llgtrn/EstateOps` | DEVELOPMENT_CELL | ESTATE | DISCOVERED |
| D252 | `Qloapps/QloApps` | `llgtrn/bnbOps` | DEVELOPMENT_CELL | HOSPITALITY | DISCOVERED |
| D253 | `QuantConnect/Lean` | `llgtrn/TradingOps` | DEVELOPMENT_CELL | TRADING | DISCOVERED |
| D254 | `quickfix/quickfix` | `llgtrn/TradingOps` | DEVELOPMENT_CELL | TRADING | DISCOVERED |
| D255 | `ray-project/ray` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | SOURCE_PRESENT |
| D256 | `RedpandaData/redpanda` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | EVENTING | DISCOVERED |
| D257 | `refly-ai/refly` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | SOURCE_PRESENT |
| D258 | `remotion-dev/remotion` | `llgtrn/W2AOps` | DEVELOPMENT_CELL | WEB2APP | SOURCE_PRESENT |
| D259 | `restatedev/restate` | `llgtrn/Chronica` | CHRONICA_FOUNDATION | EXECUTION | ABSORBING |
| D260 | `RisingWaveLabs/risingwave` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | EVENTING | DISCOVERED |
| D261 | `rook/rook` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | PLATFORM_ENGINEERING | SOURCE_PRESENT |
| D262 | `ros-controls/ros2_control` | `llgtrn/RoboticsOps` | DEVELOPMENT_CELL | ROBOTICS | DISCOVERED |
| D263 | `ros-navigation/navigation2` | `llgtrn/RoboticsOps` | DEVELOPMENT_CELL | ROBOTICS | SOURCE_PRESENT |
| D264 | `ros2/ros2` | `llgtrn/RoboticsOps` | DEVELOPMENT_CELL | ROBOTICS | SOURCE_PRESENT |
| D265 | `rudderlabs/rudder-server` | `llgtrn/ECOps` | DEVELOPMENT_CELL | COMMERCE | DISCOVERED |
| D266 | `run-llama/llama_index` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | MEMORY | DISCOVERED |
| D267 | `saleor/saleor` | `llgtrn/ECOps` | DEVELOPMENT_CELL | COMMERCE | DISCOVERED |
| D268 | `searxng/searxng` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | DISCOVERED |
| D269 | `seldonio/seldon-core` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | MODEL_SERVING | DISCOVERED |
| D270 | `ServiceNow/BrowserGym` | `llgtrn/W2AOps` | DEVELOPMENT_CELL | WEB2APP | SOURCE_PRESENT |
| D271 | `sherlock-project/sherlock` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | ABSORBING |
| D272 | `siderolabs/talos` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | CLOUD | DISCOVERED |
| D273 | `sigstore/cosign` | `llgtrn/TradeOps` | DEVELOPMENT_CELL | IDENTITY | DISCOVERED |
| D274 | `Skyvern-AI/skyvern` | `llgtrn/W2AOps` | DEVELOPMENT_CELL | WEB2APP | SOURCE_PRESENT |
| D275 | `smicallef/spiderfoot` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | ABSORBING |
| D276 | `solidusio/solidus` | `llgtrn/ECOps` | DEVELOPMENT_CELL | COMMERCE | DISCOVERED |
| D277 | `soxoj/maigret` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | ABSORBING |
| D278 | `spiffe/spire` | `llgtrn/TradeOps` | DEVELOPMENT_CELL | IDENTITY | DISCOVERED |
| D279 | `spree/spree` | `llgtrn/ECOps` | DEVELOPMENT_CELL | COMMERCE | DISCOVERED |
| D280 | `stephane/libmodbus` | `llgtrn/IndustrialOps` | DEVELOPMENT_CELL | INDUSTRIAL | DISCOVERED |
| D281 | `SteveMacenski/slam_toolbox` | `llgtrn/RoboticsOps` | DEVELOPMENT_CELL | ROBOTICS | DISCOVERED |
| D282 | `SuiteCRM/SuiteCRM-Core` | `llgtrn/CustomerOps` | DEVELOPMENT_CELL | CUSTOMER | SOURCE_PRESENT |
| D283 | `sundowndev/PhoneInfoga` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | ABSORBING |
| D284 | `supabase/supabase` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DURABLE_STATE | SOURCE_PRESENT |
| D285 | `SWE-agent/SWE-agent` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | DISCOVERED |
| D286 | `Sylius/Sylius` | `llgtrn/ECOps` | DEVELOPMENT_CELL | COMMERCE | DISCOVERED |
| D287 | `TauricResearch/TradingAgents` | `llgtrn/TradingOps` | DEVELOPMENT_CELL | TRADING | DISCOVERED |
| D288 | `tektoncd/pipeline` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | SERVERLESS | DISCOVERED |
| D289 | `temporalio/temporal` | `llgtrn/Chronica` | REFERENCE_ONLY | EXECUTION | REFERENCE_ONLY |
| D290 | `tesseract-ocr/tesseract` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | DOCUMENT | DISCOVERED |
| D291 | `thingsboard/thingsboard` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | EDGE | DISCOVERED |
| D292 | `ToolJet/ToolJet` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | SOURCE_PRESENT |
| D293 | `traccar/traccar` | `llgtrn/FleetOps` | DEVELOPMENT_CELL | FLEET | DISCOVERED |
| D294 | `traefik/traefik` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | NETWORK | DISCOVERED |
| D295 | `triton-inference-server/server` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | MODEL_SERVING | DISCOVERED |
| D296 | `twentyhq/twenty` | `llgtrn/CustomerOps` | DEVELOPMENT_CELL | CUSTOMER | DISCOVERED |
| D297 | `typesense/typesense` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DURABLE_STATE | DISCOVERED |
| D298 | `uber/h3` | `llgtrn/FleetOps` | DEVELOPMENT_CELL | LOGISTICS | DISCOVERED |
| D299 | `unclecode/crawl4ai` | `llgtrn/W2AOps` | DEVELOPMENT_CELL | AI_RUNTIME | SOURCE_PRESENT |
| D300 | `unit8co/darts` | `llgtrn/bnbOps` | DEVELOPMENT_CELL | HOSPITALITY | DISCOVERED |
| D301 | `Unleash/unleash` | `llgtrn/ECOps` | DEVELOPMENT_CELL | COMMERCE | DISCOVERED |
| D302 | `valhalla/valhalla` | `llgtrn/FleetOps` | DEVELOPMENT_CELL | LOGISTICS | DISCOVERED |
| D303 | `valkey-io/valkey` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DURABLE_STATE | DISCOVERED |
| D304 | `vendure-ecommerce/vendure` | `llgtrn/ECOps` | DEVELOPMENT_CELL | COMMERCE | DISCOVERED |
| D305 | `vercel-labs/agent-browser` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OTHER | DISCOVERED |
| D306 | `vitessio/vitess` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DURABLE_STATE | DISCOVERED |
| D307 | `vllm-project/vllm` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | AI_RUNTIME | SOURCE_PRESENT |
| D308 | `vnpy/vnpy` | `llgtrn/TradingOps` | DEVELOPMENT_CELL | TRADING | DISCOVERED |
| D309 | `volcano-sh/volcano` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | SERVERLESS | DISCOVERED |
| D310 | `VROOM-Project/vroom` | `llgtrn/FleetOps` | DEVELOPMENT_CELL | LOGISTICS | DISCOVERED |
| D311 | `wasmCloud/wasmCloud` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | CONTAINER | DISCOVERED |
| D312 | `weaviate/weaviate` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DURABLE_STATE | DISCOVERED |
| D313 | `XKNX/xknx` | `llgtrn/TwinOps` | DEVELOPMENT_CELL | BUILDING | DISCOVERED |
| D314 | `yaojingang/GEOFlow` | `llgtrn/TradeOps` | DEVELOPMENT_CELL | LOGISTICS | DISCOVERED |
| D315 | `Z4nzu/hackingtool` | `llgtrn/OSINTOps` | DEVELOPMENT_CELL | OSINT | ABSORBING |
| D316 | `zed-industries/zed` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | DEVELOPER_TOOLING | DISCOVERED |
| D317 | `zephyrproject-rtos/zephyr` | `llgtrn/EdgeOps` | DEVELOPMENT_CELL | EDGE | DISCOVERED |
| D318 | `open62541/open62541` | `llgtrn/IndustrialOps` | DEVELOPMENT_CELL | INDUSTRIAL | DISCOVERED |
| D319 | `facebookresearch/vjepa2` | `llgtrn/Chronica` | CHRONICA_ORGANISM | AI_RUNTIME | DISCOVERED |
| D320 | `danijar/dreamerv3` | `llgtrn/Chronica` | CHRONICA_ORGANISM | AI_RUNTIME | DISCOVERED |
| D321 | `gazebosim/gz-sim` | `llgtrn/SimulationOps` | DEVELOPMENT_CELL | SIMULATION | DISCOVERED |
| D322 | `apache/fineract` | `llgtrn/BankingOps` | DEVELOPMENT_CELL | BANKING | DISCOVERED |
| D323 | `element-hq/synapse` | `llgtrn/PlatformOps` | DEVELOPMENT_CELL | OTHER | DISCOVERED |

## Invariants

- Every donor has exactly one primary absorber.
- Every active Development Cell has at least one donor in this corpus.
- Donor count is not a success metric; new donors are admitted only for a proven uncovered capability gap.
- `REFERENCE_ONLY` donors are accounting/comparison evidence and may not trigger feature implementation.
- Development Cells absorb domain/provider/tooling behavior; only proven universal semantics may later converge into Chronica.
- Donor source is temporary feedstock. Production code must not depend on donor source, and absorbed source is deleted slice-by-slice.
