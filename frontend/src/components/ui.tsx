import { useState, type ReactNode } from "react";
import { Link } from "react-router-dom";
import {
  AlertCircle,
  ArrowRight,
  Clock3,
  FileCheck2,
  Gift,
  ListTodo,
  LockKeyhole,
  Ticket,
  Users,
} from "lucide-react";
import type { Campaign } from "../lib/types";
import { demo, chain, secureUrl } from "../lib/config";
import { samples, sampleDays } from "../lib/demo";
import { useApp } from "../lib/context";
import { Reward, staticReward } from "../lib/reward";
import { ApiError } from "../lib/api";
export function Brand() {
  return (
    <Link className="brand" to="/" aria-label="Oppor home">
      <img src="/favicon.svg" width="34" height="34" alt="" />
      <span>oppor</span>
    </Link>
  );
}
export function TokenArt({ index = 0 }: { index?: number }) {
  return (
    <span className={`token-art art-${index % 6}`} aria-hidden="true">
      <svg viewBox="0 0 64 64">
        {index % 6 === 0 ? (
          <>
            <ellipse
              cx="32"
              cy="32"
              rx="23"
              ry="9"
              transform="rotate(-35 32 32)"
              fill="#c7c5ff"
            />
            <circle cx="32" cy="31" r="15" fill="#b3aaff" opacity=".8" />
            <ellipse
              cx="32"
              cy="32"
              rx="23"
              ry="9"
              transform="rotate(-35 32 32)"
              fill="none"
              stroke="#eeeaff"
              opacity=".7"
            />
          </>
        ) : index % 6 === 1 ? (
          <>
            <path
              d="M14 14 27 21 38 21 50 14 49 38Q47 51 32 51T15 38Z"
              fill="#d3ccff"
            />
            <circle cx="25" cy="34" r="2" fill="#7064ee" />
            <circle cx="39" cy="34" r="2" fill="#7064ee" />
            <path d="m30 40 2 2 3-2" fill="none" stroke="#7064ee" />
          </>
        ) : index % 6 === 2 ? (
          <>
            <path
              d="m32 12 10 10-10 10-10-10ZM20 24l10 10-10 10-10-10ZM44 24l10 10-10 10-10-10ZM32 36l10 10-10 10-10-10Z"
              fill="#ece8ff"
            />
          </>
        ) : index % 6 === 3 ? (
          <>
            <path d="m13 48 17-29 19 29Z" fill="#e9e5ff" />
            <path d="m33 48 11-22 13 22Z" fill="#bdb7ff" />
            <path d="m25 48 11-18 8 18Z" fill="#9388ff" />
          </>
        ) : index % 6 === 4 ? (
          <path d="m32 11 6 16 16 5-16 6-6 16-6-16-16-6 16-5Z" fill="#d8ceff" />
        ) : (
          <>
            <path d="m32 12 18 10v21L32 53 14 43V22Z" fill="#bfb7ff" />
            <path d="m32 23 9 5v10l-9 5-9-5V28Z" fill="#5545ed" />
            <path d="m14 22 18-10v11l-9 5Z" fill="#eeeaff" />
            <path d="m41 38 9 5-18 10V43Z" fill="#eeeaff" />
          </>
        )}
      </svg>
    </span>
  );
}
export function OrbitArt() {
  return (
    <svg className="orbit-art" viewBox="0 0 380 180" aria-hidden="true">
      <defs>
        <linearGradient id="orb" x2=".8" y2="1">
          <stop stopColor="#9b93ff" />
          <stop offset="1" stopColor="#3c29de" />
        </linearGradient>
      </defs>
      <circle cx="30" cy="139" r="18" fill="#d9d7ff" />
      <circle cx="84" cy="160" r="5" fill="#bcb6ff" />
      <circle cx="348" cy="29" r="9" fill="#d4d0ff" />
      <ellipse
        cx="203"
        cy="93"
        rx="146"
        ry="40"
        transform="rotate(-10 203 93)"
        fill="none"
        stroke="#7665ff"
      />
      <circle cx="221" cy="100" r="44" fill="url(#orb)" />
      <ellipse
        cx="211"
        cy="89"
        rx="107"
        ry="48"
        transform="rotate(-30 211 89)"
        fill="none"
        stroke="#5543ff"
        strokeDasharray="2 5"
      />
      <path d="M72 100q30 65 228-5" fill="none" stroke="#bbb1ff" />
      <circle cx="156" cy="47" r="10" fill="url(#orb)" />
      <circle cx="326" cy="111" r="14" fill="url(#orb)" />
    </svg>
  );
}
export function campaignIndex(c: Campaign) {
  const i = samples.findIndex((s) => s.id === c.id);
  return i < 0 ? c.title.length % 6 : i;
}
export const rewardLabel = staticReward;
export function endLabel(c: Campaign) {
  const i = samples.findIndex((s) => s.id === c.id);
  if (demo && i >= 0)
    return `Ends in ${sampleDays[i]} ${sampleDays[i] === 1 ? "day" : "days"}`;
  const days = Math.ceil((Date.parse(c.cutoff_at) - Date.now()) / 86400000);
  return days <= 0
    ? "Entries closed"
    : `Ends in ${days} ${days === 1 ? "day" : "days"}`;
}
export function Badge({
  children,
  tone = "indigo",
}: {
  children: ReactNode;
  tone?: string;
}) {
  return <span className={`badge ${tone}`}>{children}</span>;
}
export function CampaignCard({ campaign: c }: { campaign: Campaign }) {
  const funded = [
    "ACTIVE",
    "REVIEWING",
    "ELIGIBILITY_LOCKED",
    "ALLOCATION_READY",
    "CLAIM_OPEN",
    "COMPLETED",
  ].includes(c.status);
  return (
    <Link className="campaign-card" to={`/campaigns/${c.id}`}>
      <div className="card-top">
        <TokenArt index={campaignIndex(c)} />
        <div>
          <span className="chain-tag">Arc</span>
          <h3>{c.title}</h3>
          <p>{c.description}</p>
        </div>
      </div>
      <div className="card-reward">
        <div>
          <strong>
            <Reward campaign={c} />
          </strong>
          <span>Total reward pool</span>
        </div>
        <Badge tone={funded ? "indigo" : "neutral"}>
          <LockKeyhole size={15} />
          {funded
            ? "Reward funded"
            : c.status.replaceAll("_", " ").toLowerCase()}
        </Badge>
      </div>
      <div className="card-meta">
        <span>
          {c.distribution.mode === "RAFFLE" ? <Ticket /> : <Users />}
          {c.distribution.mode === "RAFFLE" ? "Raffle" : "All eligible"}
        </span>
        <span>
          <ListTodo />
          {c.tasks?.length ?? c.rules?.tasks.length ?? "—"} tasks
        </span>
        <span>
          <Clock3 />
          {endLabel(c)}
        </span>
      </div>
    </Link>
  );
}
export function FlowStrip() {
  return (
    <div className="flow-strip">
      <span className="flow-icon">
        <ListTodo />
      </span>
      <div>
        <strong>
          Complete tasks → Submit evidence → Creator review → Claim rewards
        </strong>
        <p>Rewards depend on campaign rules. Review happens after cutoff.</p>
      </div>
      {demo && <small>Concept UI · Sample campaigns</small>}
    </div>
  );
}
export function PageHeader({
  title,
  description,
  action,
}: {
  title: string;
  description?: string;
  action?: ReactNode;
}) {
  return (
    <header className="page-heading">
      <div>
        <h1>{title}</h1>
        {description && <p>{description}</p>}
      </div>
      {action}
    </header>
  );
}
export function Loading() {
  return (
    <div className="state" role="status">
      <span className="spinner" />
      <p>Loading campaigns…</p>
    </div>
  );
}
export function ErrorState({
  error,
  retry,
}: {
  error: unknown;
  retry?: () => void;
}) {
  return (
    <div className="state error" role="alert">
      <AlertCircle />
      <h2>We couldn’t complete this request</h2>
      <p>{error instanceof Error ? error.message : "Please try again."}</p>
      {error instanceof ApiError && error.requestId && (
        <small>Reference: {error.requestId}</small>
      )}
      {retry && (
        <button className="button outline" onClick={retry}>
          Try again
        </button>
      )}
    </div>
  );
}
export function Empty({
  title,
  description,
  action,
}: {
  title: string;
  description: string;
  action?: ReactNode;
}) {
  return (
    <div className="state">
      <Gift size={36} />
      <h2>{title}</h2>
      <p>{description}</p>
      {action}
    </div>
  );
}
export function AuthGate({ children }: { children: ReactNode }) {
  const { session, connect } = useApp();
  return session ? (
    <>{children}</>
  ) : (
    <Empty
      title="Your workspace starts here"
      description="Connect your wallet to view your entries, rewards, and campaigns."
      action={
        <button className="button primary" onClick={connect}>
          Connect wallet <ArrowRight size={17} />
        </button>
      }
    />
  );
}
export function DemoNotice() {
  return demo ? (
    <div className="notice">
      <FileCheck2 size={19} />
      <span>
        Demo mode. Campaigns, entries, and drafts are examples; no real rewards
        or transactions.
      </span>
    </div>
  ) : null;
}
export function SafeLink({
  href,
  children,
}: {
  href: string | null | undefined;
  children: ReactNode;
}) {
  if (!href) return null;
  try {
    const url = secureUrl(href, import.meta.env.DEV);
    return (
      <a
        href={url}
        target="_blank"
        rel="noopener noreferrer"
        className="text-link"
      >
        {children} ↗
      </a>
    );
  } catch {
    return <span className="muted">Link unavailable</span>;
  }
}
export function ExplorerLink({ hash }: { hash: string }) {
  return (
    <SafeLink href={`${chain.blockExplorers.default.url}/tx/${hash}`}>
      {hash.slice(0, 12)}…
    </SafeLink>
  );
}
export function ActionButton({
  action,
  children,
  className = "button primary",
  disabled = false,
  onDone,
}: {
  action: () => Promise<unknown>;
  children: ReactNode;
  className?: string;
  disabled?: boolean;
  onDone?: () => void;
}) {
  const [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  return (
    <div className="action">
      <button
        className={className}
        disabled={busy || disabled}
        onClick={async () => {
          setBusy(true);
          setError("");
          try {
            await action();
            onDone?.();
          } catch (e) {
            setError(e instanceof Error ? e.message : "The action failed.");
          } finally {
            setBusy(false);
          }
        }}
      >
        {busy ? (
          <>
            <span className="spinner small" />
            Working…
          </>
        ) : (
          children
        )}
      </button>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
