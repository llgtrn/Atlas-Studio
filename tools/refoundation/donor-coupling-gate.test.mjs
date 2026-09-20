import { test } from 'node:test'
import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { findDonorCouplingViolations } from './donor-coupling-gate.mjs'

function makeFixtureRepo() {
  const root = mkdtempSync(join(tmpdir(), 'donor-coupling-gate-'))
  const git = (args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' })
  git(['init', '--quiet'])
  git(['config', 'user.email', 'test@example.com'])
  git(['config', 'user.name', 'Test'])
  return { root, git }
}

function writeAndCommit(root, git, files, message) {
  for (const [path, content] of Object.entries(files)) {
    const full = join(root, path)
    mkdirSync(join(full, '..'), { recursive: true })
    writeFileSync(full, content)
  }
  git(['add', '-A'])
  git(['commit', '--quiet', '-m', message])
  return git(['rev-parse', 'HEAD']).trim()
}

test('a Go free function deleted while a sibling file in the same package still calls it is flagged', () => {
  const { root, git } = makeFixtureRepo()
  try {
    writeAndCommit(
      root,
      git,
      {
        'temporary/donor/pkg/widget/widget.go': 'package widget\n\nfunc NewWidget() *Widget {\n\treturn &Widget{}\n}\n',
        'temporary/donor/pkg/widget/user.go': 'package widget\n\nfunc UseIt() {\n\tw := NewWidget()\n\t_ = w\n}\n',
      },
      'add widget package',
    )
    const before = git(['rev-parse', 'HEAD']).trim()
    execFileSync('git', ['rm', '--quiet', 'temporary/donor/pkg/widget/widget.go'], { cwd: root })
    git(['commit', '--quiet', '-m', 'delete widget.go, leaving user.go coupled'])

    const violations = findDonorCouplingViolations(before, root)
    assert.equal(violations.length, 1)
    assert.equal(violations[0].donor, 'donor')
    assert.equal(violations[0].deletedPath, 'temporary/donor/pkg/widget/widget.go')
    assert.equal(violations[0].referencingFile, 'temporary/donor/pkg/widget/user.go')
    assert.match(violations[0].reason, /NewWidget/)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('a Go exported symbol deleted while a file in a DIFFERENT package calls it package-qualified is flagged', () => {
  const { root, git } = makeFixtureRepo()
  try {
    writeAndCommit(
      root,
      git,
      {
        'temporary/donor/pkg/idpool/idpool.go': 'package idpool\n\ntype ID uint64\n\nfunc NewIDPool() *IDPool {\n\treturn &IDPool{}\n}\n',
        'temporary/donor/pkg/allocator/allocator.go':
          'package allocator\n\nimport "donor/pkg/idpool"\n\nfunc New() {\n\tp := idpool.NewIDPool()\n\t_ = p\n}\n',
      },
      'add idpool + allocator',
    )
    const before = git(['rev-parse', 'HEAD']).trim()
    execFileSync('git', ['rm', '--quiet', 'temporary/donor/pkg/idpool/idpool.go'], { cwd: root })
    git(['commit', '--quiet', '-m', 'delete idpool.go, leaving allocator.go coupled'])

    const violations = findDonorCouplingViolations(before, root)
    assert.equal(violations.length, 1)
    assert.equal(violations[0].referencingFile, 'temporary/donor/pkg/allocator/allocator.go')
    assert.match(violations[0].reason, /NewIDPool/)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('an unexported Go symbol used only cross-package is NOT flagged (it could not have compiled that way -- false match, not real coupling)', () => {
  const { root, git } = makeFixtureRepo()
  try {
    writeAndCommit(
      root,
      git,
      {
        'temporary/donor/pkg/a/a.go': 'package a\n\nfunc helper() int {\n\treturn 1\n}\n',
        'temporary/donor/pkg/b/b.go': 'package b\n\nfunc helper() int {\n\treturn 2\n}\n',
      },
      'two unrelated unexported helper() funcs in different packages',
    )
    const before = git(['rev-parse', 'HEAD']).trim()
    execFileSync('git', ['rm', '--quiet', 'temporary/donor/pkg/a/a.go'], { cwd: root })
    git(['commit', '--quiet', '-m', 'delete pkg/a/a.go'])

    const violations = findDonorCouplingViolations(before, root)
    assert.deepEqual(violations, [])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('a generic exported Go name (e.g. Split) in an unrelated package is NOT flagged just because the deleted file also defined a Split', () => {
  const { root, git } = makeFixtureRepo()
  try {
    writeAndCommit(
      root,
      git,
      {
        'temporary/donor/shamir/shamir.go': 'package shamir\n\nfunc Split() {}\n',
        'temporary/donor/strutil/strutil.go': 'package strutil\n\nimport "strings"\n\nfunc Do() {\n\tstrings.Split("a,b", ",")\n}\n',
      },
      'unrelated Split in a different package',
    )
    const before = git(['rev-parse', 'HEAD']).trim()
    execFileSync('git', ['rm', '--quiet', 'temporary/donor/shamir/shamir.go'], { cwd: root })
    git(['commit', '--quiet', '-m', 'delete shamir.go'])

    const violations = findDonorCouplingViolations(before, root)
    assert.deepEqual(violations, [])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('a receiver method (e.g. String()) is never used as a coupling signal, even when it collides with many unrelated types', () => {
  const { root, git } = makeFixtureRepo()
  try {
    writeAndCommit(
      root,
      git,
      {
        'temporary/donor/cmd/httprange.go': 'package cmd\n\ntype HTTPRangeSpec struct{}\n\nfunc (h *HTTPRangeSpec) String() string {\n\treturn ""\n}\n',
        'temporary/donor/cmd/unrelated.go': 'package cmd\n\ntype OtherThing struct{}\n\nfunc (o *OtherThing) String() string {\n\treturn "other"\n}\n',
      },
      'two unrelated String() methods in the same package',
    )
    const before = git(['rev-parse', 'HEAD']).trim()
    execFileSync('git', ['rm', '--quiet', 'temporary/donor/cmd/httprange.go'], { cwd: root })
    git(['commit', '--quiet', '-m', 'delete httprange.go'])

    const violations = findDonorCouplingViolations(before, root)
    assert.deepEqual(violations, [])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('a Rust file deleted while a sibling mod.rs still declares `mod <it>;` is flagged as a dangling module declaration', () => {
  const { root, git } = makeFixtureRepo()
  try {
    writeAndCommit(
      root,
      git,
      {
        'temporary/donor/crates/x/src/retry_after.rs': 'pub fn compute() -> u32 {\n\t1\n}\n',
        'temporary/donor/crates/x/src/mod.rs': 'mod retry_after;\nmod other;\n',
      },
      'add retry_after module',
    )
    const before = git(['rev-parse', 'HEAD']).trim()
    execFileSync('git', ['rm', '--quiet', 'temporary/donor/crates/x/src/retry_after.rs'], { cwd: root })
    git(['commit', '--quiet', '-m', 'delete retry_after.rs, leaving mod.rs dangling'])

    const violations = findDonorCouplingViolations(before, root)
    assert.equal(violations.length, 1)
    assert.equal(violations[0].referencingFile, 'temporary/donor/crates/x/src/mod.rs')
    assert.match(violations[0].reason, /dangling module declaration/)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('a non-code file (e.g. sgml) deleted while another file still references its exact filename is flagged generically', () => {
  const { root, git } = makeFixtureRepo()
  try {
    writeAndCommit(
      root,
      git,
      {
        'temporary/donor/doc/mvcc.sgml': '<para>mvcc chapter</para>\n',
        'temporary/donor/doc/filelist.sgml': '<!ENTITY mvcc SYSTEM "mvcc.sgml">\n',
      },
      'add mvcc.sgml + filelist entity',
    )
    const before = git(['rev-parse', 'HEAD']).trim()
    execFileSync('git', ['rm', '--quiet', 'temporary/donor/doc/mvcc.sgml'], { cwd: root })
    git(['commit', '--quiet', '-m', 'delete mvcc.sgml, leaving filelist.sgml dangling'])

    const violations = findDonorCouplingViolations(before, root)
    assert.equal(violations.length, 1)
    assert.equal(violations[0].referencingFile, 'temporary/donor/doc/filelist.sgml')
    assert.match(violations[0].reason, /mvcc\.sgml/)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('deleting a file with no remaining referencers anywhere in the donor tree is clean (a genuinely safe drain)', () => {
  const { root, git } = makeFixtureRepo()
  try {
    writeAndCommit(
      root,
      git,
      {
        'temporary/donor/pkg/leaf/leaf.go': 'package leaf\n\nfunc Standalone() {}\n',
        'temporary/donor/pkg/other/other.go': 'package other\n\nfunc DoesNotUseLeaf() {}\n',
      },
      'add an unrelated leaf package',
    )
    const before = git(['rev-parse', 'HEAD']).trim()
    execFileSync('git', ['rm', '--quiet', 'temporary/donor/pkg/leaf/leaf.go'], { cwd: root })
    git(['commit', '--quiet', '-m', 'delete leaf.go -- nothing depends on it'])

    const violations = findDonorCouplingViolations(before, root)
    assert.deepEqual(violations, [])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('deleting the whole donor directory (donor fully drained) never flags -- there is no retained tree left to dangle', () => {
  const { root, git } = makeFixtureRepo()
  try {
    writeAndCommit(
      root,
      git,
      {
        'temporary/donor/pkg/a/a.go': 'package a\n\nfunc Foo() {}\n',
        'temporary/donor/pkg/b/b.go': 'package b\n\nimport "donor/pkg/a"\n\nfunc Bar() {\n\ta.Foo()\n}\n',
      },
      'a small donor about to be fully removed',
    )
    const before = git(['rev-parse', 'HEAD']).trim()
    execFileSync('git', ['rm', '-r', '--quiet', 'temporary/donor'], { cwd: root })
    git(['commit', '--quiet', '-m', 'donor fully retired'])

    const violations = findDonorCouplingViolations(before, root)
    assert.deepEqual(violations, [])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('the live repo, as of the donor-restoration fix commit, has zero coupling violations since the immediately preceding commit', () => {
  const violations = findDonorCouplingViolations('HEAD~1')
  assert.deepEqual(violations, [])
})
