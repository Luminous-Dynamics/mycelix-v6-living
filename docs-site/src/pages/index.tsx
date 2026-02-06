import React from 'react';
import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import styles from './index.module.css';

type FeatureItem = {
  title: string;
  description: string;
  icon: string;
};

const features: FeatureItem[] = [
  {
    title: '28-Day Cycle',
    description: 'Embrace natural rhythms with four distinct phases: Dawn, Surge, Settle, and Rest. Your system adapts automatically.',
    icon: '🌙',
  },
  {
    title: '21 Living Primitives',
    description: 'Build with intelligent building blocks that understand time and context. From Pulse to Bloom, each primitive lives.',
    icon: '🍄',
  },
  {
    title: 'Distributed by Design',
    description: 'Nodes connect and collaborate like mycelial threads. Scale horizontally with built-in coordination.',
    icon: '🌐',
  },
  {
    title: 'Phase-Aware APIs',
    description: 'WebSocket, REST, and GraphQL APIs that adapt to the current phase. Rate limits, timeouts, and behavior shift organically.',
    icon: '⚡',
  },
  {
    title: 'Multi-Language SDKs',
    description: 'First-class support for TypeScript, Python, and Go. Each SDK feels native to its language.',
    icon: '📦',
  },
  {
    title: 'Production Ready',
    description: 'Battle-tested in production. Kubernetes-native, observable, and secure by default.',
    icon: '🛡️',
  },
];

function Feature({ title, description, icon }: FeatureItem) {
  return (
    <div className={clsx('col col--4')}>
      <div className={styles.feature}>
        <div className={styles.featureIcon}>{icon}</div>
        <h3>{title}</h3>
        <p>{description}</p>
      </div>
    </div>
  );
}

function HomepageHeader() {
  const { siteConfig } = useDocusaurusContext();
  return (
    <header className={clsx('hero hero--primary', styles.heroBanner)}>
      <div className="container">
        <h1 className="hero__title">{siteConfig.title}</h1>
        <p className="hero__subtitle">{siteConfig.tagline}</p>
        <div className={styles.buttons}>
          <Link
            className="button button--secondary button--lg"
            to="/docs/intro">
            Get Started
          </Link>
          <Link
            className="button button--outline button--lg"
            to="/docs/quick-start">
            5-Minute Tutorial
          </Link>
        </div>
      </div>
    </header>
  );
}

function CodeExample() {
  return (
    <div className={styles.codeSection}>
      <div className="container">
        <div className="row">
          <div className="col col--6">
            <h2>Build Living Systems</h2>
            <p>
              Create primitives that adapt to the 28-day cycle. Your code
              automatically adjusts behavior based on the current phase.
            </p>
            <ul>
              <li><strong>Dawn</strong>: Gentle initialization, high fault tolerance</li>
              <li><strong>Surge</strong>: Maximum throughput, aggressive optimization</li>
              <li><strong>Settle</strong>: Pattern recognition, consolidation</li>
              <li><strong>Rest</strong>: Minimal activity, maintenance mode</li>
            </ul>
          </div>
          <div className="col col--6">
            <pre className={styles.codeBlock}>
{`import { Pulse, Phase } from '@mycelix/core';

const heartbeat = new Pulse({
  name: 'heartbeat',
  interval: {
    Dawn: '10s',
    Surge: '1s',
    Settle: '5s',
    Rest: '30s',
  },
  emit: async (ctx) => ({
    status: 'alive',
    phase: ctx.phase,
    uptime: process.uptime(),
  }),
});`}
            </pre>
          </div>
        </div>
      </div>
    </div>
  );
}

function QuickLinks() {
  return (
    <div className={styles.quickLinks}>
      <div className="container">
        <h2>Quick Links</h2>
        <div className="row">
          <div className="col col--3">
            <Link to="/docs/installation" className={styles.quickLink}>
              <h3>Installation</h3>
              <p>Get up and running in 2 minutes</p>
            </Link>
          </div>
          <div className="col col--3">
            <Link to="/docs/concepts/cycle" className={styles.quickLink}>
              <h3>The Cycle</h3>
              <p>Understand the 28-day rhythm</p>
            </Link>
          </div>
          <div className="col col--3">
            <Link to="/docs/concepts/primitives" className={styles.quickLink}>
              <h3>Primitives</h3>
              <p>Explore 21 building blocks</p>
            </Link>
          </div>
          <div className="col col--3">
            <Link to="/docs/api/websocket" className={styles.quickLink}>
              <h3>API Reference</h3>
              <p>WebSocket, REST, GraphQL</p>
            </Link>
          </div>
        </div>
      </div>
    </div>
  );
}

export default function Home(): React.ReactElement {
  const { siteConfig } = useDocusaurusContext();
  return (
    <Layout
      title={`${siteConfig.title} - A Living System for Distributed Intelligence`}
      description="Mycelix is a living system for distributed intelligence, inspired by mycelial networks. Build adaptive applications with the 28-day cycle and 21 living primitives.">
      <HomepageHeader />
      <main>
        <section className={styles.features}>
          <div className="container">
            <div className="row">
              {features.map((props, idx) => (
                <Feature key={idx} {...props} />
              ))}
            </div>
          </div>
        </section>
        <CodeExample />
        <QuickLinks />
      </main>
    </Layout>
  );
}
