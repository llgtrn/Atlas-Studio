# Atlas Studio UI

TypeScript/TSX frontend projection for Atlas Studio.

The frontend is not a second source of engineering truth. It will consume bounded projections from the Rust Atlas engine and evolve toward an Engineering World / World Canvas interface over ATLASX.

The intended interaction hierarchy is semantic-first:

```text
engineering world
→ technology territory
→ system
→ architecture
→ component
→ module
→ symbol
→ source
```

Traditional source editing, diff, terminal, design and agent surfaces are projections of the same Atlas semantic world rather than separate applications.
