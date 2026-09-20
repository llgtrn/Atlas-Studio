import { describe, expect, it } from 'vitest';
import { layoutAtlasNodes } from './DocsAtlasGraph';
import type { DocsAtlasNode } from '../docsAtlasModel';

function node(id: string, group: string): DocsAtlasNode {
  return {
    id,
    title: id,
    kind: 'architecture-owner',
    family: 'architecture',
    group,
    status: 'ACTIVE',
    summary: 'fixture',
    path: `docs/${id}.md`,
    headings: [],
    contractIds: [],
    runtimeOwners: [],
  };
}

describe('layoutAtlasNodes', () => {
  it('is deterministic and separates responsibility columns', () => {
    const input = [
      node('world', 'architecture/foundation'),
      node('authority', 'architecture/governance'),
      node('state', 'architecture/foundation'),
    ];
    const first = layoutAtlasNodes(input);
    const second = layoutAtlasNodes([...input].reverse());
    expect(first.nodes).toEqual(second.nodes);
    const world = first.nodes.find((item) => item.id === 'world')!;
    const authority = first.nodes.find((item) => item.id === 'authority')!;
    expect(world.x).not.toBe(authority.x);
  });
});
