---
id: atlas.standard.docs.v1
type: contract
status: active
canonical: true
---
# Documentation Standard v1

## Required Frontmatter

Every maintained Markdown file under .atlas carries id, type, status and canonical. Missing frontmatter is a hard violation.

## Required Control Documents

Every managed repository contains the same eleven control paths: README, INDEX, TEMPLATE, architecture router, NORTH-STAR, SYSTEM architecture, SYSTEM-BLUEPRINT, SYSTEM-CONTRACT, decisions index, DEVELOPMENT guide and references index.

## Authoring Rules

There are no repository-specific documentation exceptions. Extra domain docs are allowed but obey the same metadata, links, canonicality and supersession rules. Analysis may explain a documentation gap, but work preparation refuses coding until DocsGate is READY.
