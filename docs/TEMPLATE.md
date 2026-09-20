---
id: atlas.docs.template
type: contract
status: active
canonical: true
---
# Atlas Documentation Template

## Required Frontmatter

Every maintained Markdown document under docs has id, type, status, and canonical frontmatter.

## Required Control Documents

Every Atlas-managed repository must contain the same eleven control-document paths required by atlas.docs.v1. No repository class may omit or rename one.

## Authoring Rules

Documentation must state durable responsibility before implementation. North Star defines destination, architecture defines ownership and boundaries, blueprint defines composition, contracts define hard invariants, and the development guide defines the coding procedure. Git history is archive; maintained docs describe the current system.
