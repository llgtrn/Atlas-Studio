/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under the BSD-style license found in the
 * LICENSE file in the root directory of this source tree.
 *
 * @format
 */

import React from 'react';
import clsx from 'clsx';
import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import useBaseUrl from '@docusaurus/useBaseUrl';
import styles from './styles.module.css';

const features = [
  {
    title: 'Semantic code graph',
    description: (
      <>
        Definitions, references, call graphs, type hierarchies, and
        cross-language links — not text matching.
      </>
    ),
  },
  {
    title: 'Multi-language',
    description: (
      <>
        Indexers for C++, Hack, Python, Haskell, Flow, .NET, Go, Java, Rust, and
        TypeScript.
      </>
    ),
  },
  {
    title: 'Built for scale',
    description: (
      <>
        Compact, incremental storage designed to index monorepos with billions
        of facts.
      </>
    ),
  },
  {
    title: 'Angle query language',
    description: (
      <>
        A typed, declarative query language for composing precise questions over
        the code graph.
      </>
    ),
  },
  {
    title: 'Agent- and tool-friendly',
    description: (
      <>
        Query via CLI, language-specific APIs or Thrift — ideal for IDEs, code
        review bots, refactoring tools, LLM coding agents, and more.
      </>
    ),
  },
  {
    title: 'Extensible schemas',
    description: (
      <>
        Define your own predicates to capture domain-specific facts about your
        language or codebase.
      </>
    ),
  },
];

function Feature({imageUrl, title, description}) {
  const imgUrl = useBaseUrl(imageUrl);
  return (
    <div className={clsx('col col--4', styles.feature)}>
      {imgUrl && (
        <div className="text--center">
          <img className={styles.featureImage} src={imgUrl} alt={title} />
        </div>
      )}
      <h3>{title}</h3>
      <p>{description}</p>
    </div>
  );
}

function Home() {
  const context = useDocusaurusContext();
  const {siteConfig = {}} = context;
  return (
    <Layout
      title={`${siteConfig.title}`}
      description="System for collecting, deriving and querying facts about source code">
      <header className={clsx('hero hero--primary', styles.heroBanner)}>
        <div className="container">
          <img
            className={styles.heroLogo}
            src="img/icon.png"
            alt="Glean Logo"
            width="170"
          />
          <h1 className="hero__title">{siteConfig.title}</h1>
          <p className="hero__subtitle">{siteConfig.tagline}</p>
          <div className={styles.buttons}>
            <Link
              className={clsx(
                'button button--outline button--secondary button--lg glean',
                styles.buttons,
              )}
              to={useBaseUrl('docs/introduction')}>
              Get Started
            </Link>
          </div>
        </div>
      </header>
      <main>
        <section className={styles.intro}>
          <div className="container padding-vert--xl">
            <h2 className="text--center">What is Glean?</h2>
            <p className={styles.introText}>
              <strong>Glean</strong> is an open-source code indexing system that
              stores typed, schema-defined facts about source code in a
              queryable database. Facts cover definitions, references, types,
              call relationships, inheritance, imports, and more. Facts can be
              queried with Angle, a Datalog-style query language. They are
              produced by indexers for languages including C++, Hack, Python,
              Haskell, and Flow, plus LSIF/SCIP support for Go, Java, Rust, and
              TypeScript.
            </p>
            <p className={styles.introText}>
              Use Glean when you need <em>precise, semantic</em> answers about
              code rather than text-based guesses. Typical questions Glean
              answers directly:
            </p>
            <ul className={styles.introText}>
              <li>
                <em>“Where is this symbol defined?”</em>
              </li>
              <li>
                <em>“Who calls this function?”</em>
              </li>
              <li>
                <em>“What implements this interface?”</em>
              </li>
              <li>
                <em>“What does this type alias resolve to?”</em>
              </li>
              <li>
                <em>“What are the transitive dependencies of this module?”</em>
              </li>
            </ul>
            <p className={styles.introText}>
              Coding agents, IDEs, and developer tools query Glean instead of
              relying on grep when they need accuracy, cross-file/cross-language
              reasoning, or large-scale code analysis.
            </p>
          </div>
        </section>
        <div className="container padding-vert--xl text--left">
          <div className="row">
            <div className="col">
              <h1 className="text--center">Key Features</h1>
              {features && features.length > 0 && (
                <section className={styles.features}>
                  <div className="container">
                    <div className="row">
                      {features.map(({title, imageUrl, description}) => (
                        <Feature
                          key={title}
                          title={title}
                          imageUrl={imageUrl}
                          description={description}
                        />
                      ))}
                    </div>
                  </div>
                </section>
              )}
            </div>
          </div>
        </div>
        <section className={styles.usecases}>
          <div className="container padding-vert--xl">
            <h2 className="text--center">When to use Glean</h2>
            <ul className={styles.usecaseList}>
              <li>
                <strong>Code navigation:</strong> jump-to-definition, find
                references, call hierarchy, type hierarchy.
              </li>
              <li>
                <strong>Refactoring &amp; migrations:</strong> find every
                callsite, every implementer, every override across a monorepo.
              </li>
              <li>
                <strong>Code search agents &amp; LLMs:</strong> ground answers
                in real symbol relationships instead of grep heuristics.
              </li>
              <li>
                <strong>Dependency analysis:</strong> module/file/symbol-level
                dependency graphs and impact analysis.
              </li>
              <li>
                <strong>Code review automation:</strong> reason about what a
                change actually affects.
              </li>
              <li>
                <strong>Custom code intelligence:</strong> build new tools on
                top of a uniform, language-agnostic fact store.
              </li>
            </ul>
          </div>
        </section>
      </main>
    </Layout>
  );
}

export default Home;
