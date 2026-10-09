import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import {
  useInfiniteQuery,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { ArrowLeft, Check, ChevronRight, Settings2 } from "lucide-react";
import {
  ActionButton,
  AuthGate,
  Badge,
  DemoNotice,
  ErrorState,
  Loading,
  PageHeader,
  SafeLink,
  rewardLabel,
} from "../components/ui";
import { TransactionButton } from "../components/transaction";
import { useCampaign } from "../lib/queries";
import { useApp } from "../lib/context";
import { Dialog } from "../lib/context";
import { demo } from "../lib/config";
import { post, request } from "../lib/api";
import { shorten } from "../lib/wallet";
import type {
  AllocationPreview,
  Entry,
  EntryPage,
  EvidenceResult,
  Results,
} from "../lib/types";
import { CampaignForm } from "./create";
export function Manage() {
  return (
    <AuthGate>
      <ManageContent />
    </AuthGate>
  );
}
function ManageContent() {
  const { id = "" } = useParams();
  const query = useCampaign(id);
  const { session } = useApp();
  const client = useQueryClient();
  const [editing, setEditing] = useState(false);
  const [confirm, setConfirm] = useState<"config" | "eligibility" | null>(null);
  if (query.isPending) return <Loading />;
  if (query.isError || !query.data) return <ErrorState error={query.error} />;
  const c = query.data;
  if (session?.wallet.toLowerCase() !== c.creator_wallet.toLowerCase())
    return (
      <ErrorState
        error={new Error("Only the creator can manage this campaign.")}
      />
    );
  const stages = [
    "DRAFT",
    "CONFIG_LOCKED",
    "FUNDING",
    "ACTIVE",
    "REVIEWING",
    "ELIGIBILITY_LOCKED",
    "ALLOCATION_READY",
    "CLAIM_OPEN",
  ];
  let step = stages.indexOf(c.status);
  if (c.status === "SCHEDULED") step = 3;
  return (
    <>
      <Link className="back-link" to="/campaigns">
        <ArrowLeft size={16} />
        My campaigns
      </Link>
      <PageHeader
        title="Campaign workspace"
        description={c.title}
        action={
          <Link className="button outline" to={`/campaigns/${id}`}>
            View campaign
          </Link>
        }
      />
      <DemoNotice />
      <div className="steps">
        {[
          "Draft",
          "Deploy",
          "Fund",
          "Activate",
          "Review",
          "Allocate",
          "Finalize",
          "Claim",
        ].map((label, i) => (
          <span className={i <= step ? "current" : ""} key={label}>
            {i < step ? <Check size={14} /> : i + 1}
            <b>{label}</b>
          </span>
        ))}
      </div>
      <div className="metric-grid">
        <div className="panel">
          <p className="muted">Status</p>
          <strong>{c.status.replaceAll("_", " ")}</strong>
        </div>
        <div className="panel">
          <p className="muted">Reward pool</p>
          <strong>{rewardLabel(c)}</strong>
        </div>
        <div className="panel">
          <p className="muted">Registered entries</p>
          <strong>{c.registered_count.toLocaleString()}</strong>
        </div>
      </div>
      <section className="panel">
        <div className="section-heading">
          <div>
            <h2>Next steps</h2>
            <p className="muted">Configuration version {c.version}</p>
          </div>
          <button
            className="icon-button"
            aria-label="Refresh campaign"
            onClick={() => void query.refetch()}
          >
            <Settings2 size={20} />
          </button>
        </div>
        <div className="action-row">
          {c.status === "DRAFT" && (
            <>
              <button
                className="button outline"
                onClick={() => setEditing(!editing)}
              >
                {editing ? "Close editor" : "Edit draft"}
              </button>
              <button
                className="button primary"
                disabled={demo}
                onClick={() => setConfirm("config")}
              >
                Lock configuration
              </button>
            </>
          )}
          {c.status === "CONFIG_LOCKED" && !c.escrow_address && (
            <TransactionButton
              campaign={c}
              kind="CREATE"
              label="Deploy escrow"
            />
          )}
          {c.escrow_address &&
            ["CONFIG_LOCKED", "FUNDING"].includes(c.status) && (
              <>
                <TransactionButton
                  campaign={c}
                  kind="FUND"
                  label="Approve & fund reward pool"
                />
                <TransactionButton
                  campaign={c}
                  kind="ACTIVATE"
                  label="Activate campaign"
                />
              </>
            )}
          {["ACTIVE", "REVIEWING"].includes(c.status) && (
            <button
              className="button primary"
              disabled={demo || Date.now() < Date.parse(c.cutoff_at)}
              onClick={() => setConfirm("eligibility")}
            >
              Lock reviewed eligibility
            </button>
          )}
          {c.status === "ALLOCATION_READY" && (
            <TransactionButton
              campaign={c}
              kind="FINALIZE"
              label="Finalize reward allocation"
            />
          )}
          {c.escrow_address &&
            !["CANCELLED", "COMPLETED", "CLOSED"].includes(c.status) && (
              <TransactionButton
                campaign={c}
                kind="CANCEL"
                label="Cancel campaign"
              />
            )}
          {c.escrow_address && Date.now() > Date.parse(c.claim_deadline) && (
            <TransactionButton
              campaign={c}
              kind="SWEEP"
              label="Refund unclaimed rewards"
            />
          )}
        </div>
        {demo && (
          <p className="fineprint">
            Deployment and campaign state transitions require the live backend
            and escrow contracts.
          </p>
        )}
        {c.config_hash && (
          <p className="mono break">Configuration hash: {c.config_hash}</p>
        )}
      </section>
      {editing && c.status === "DRAFT" && (
        <CampaignForm initial={c} onSaved={() => setEditing(false)} />
      )}
      <CreatorEntries id={id} />
      {[
        "ELIGIBILITY_LOCKED",
        "ALLOCATION_READY",
        "CLAIM_OPEN",
        "COMPLETED",
      ].includes(c.status) && (
        <AllocationPanel
          id={id}
          finalized={["CLAIM_OPEN", "COMPLETED"].includes(c.status)}
        />
      )}{" "}
      {confirm && (
        <Dialog close={() => setConfirm(null)}>
          <h2 id="dialog-title">
            {confirm === "config"
              ? "Lock this configuration?"
              : "Lock reviewed eligibility?"}
          </h2>
          <p>
            {confirm === "config"
              ? "Reward, task, and deadline settings become immutable. Check every field before proceeding."
              : "All submitted entries must have a review decision. This creates an immutable eligibility snapshot and starts reward allocation."}
          </p>
          <ActionButton
            action={async () => {
              await post(
                `/campaigns/${id}/lock-${confirm === "config" ? "config" : "eligibility"}`,
                { expected_version: c.version },
              );
              await client.invalidateQueries();
              setConfirm(null);
            }}
          >
            Confirm lock <ChevronRight size={16} />
          </ActionButton>
        </Dialog>
      )}
    </>
  );
}
function CreatorEntries({ id }: { id: string }) {
  const [status, setStatus] = useState("");
  const [selected, setSelected] = useState<Entry | null>(null);
  const entries = useInfiniteQuery({
    queryKey: ["creator-entries", id, status],
    initialPageParam: null as string | null,
    queryFn: ({ pageParam }): Promise<EntryPage> =>
      demo
        ? Promise.resolve({ items: [], next_cursor: null })
        : request(
            `/campaigns/${id}/entries?limit=25${status ? `&status=${status}` : ""}${pageParam ? `&after=${encodeURIComponent(pageParam)}` : ""}`,
          ),
    getNextPageParam: (p) => p.next_cursor,
  });
  return (
    <section className="panel">
      <div className="section-heading">
        <div>
          <h2>Review entries</h2>
          <p className="muted">
            Review after cutoff, before the review deadline.
          </p>
        </div>
        <label>
          <span className="sr-only">Entry status</span>
          <select value={status} onChange={(e) => setStatus(e.target.value)}>
            <option value="">All entries</option>
            {[
              "REGISTERED",
              "SUBMITTED",
              "ELIGIBLE",
              "DISQUALIFIED",
              "NOT_SUBMITTED",
            ].map((s) => (
              <option key={s}>{s}</option>
            ))}
          </select>
        </label>
      </div>
      {entries.isPending ? (
        <Loading />
      ) : entries.isError ? (
        <ErrorState error={entries.error} />
      ) : (
        <>
          {entries.data.pages
            .flatMap((p) => p.items)
            .map((e) => (
              <button
                key={e.id}
                className="review-row"
                onClick={() => setSelected(e)}
              >
                <span className="mono">{shorten(e.payout_wallet)}</span>
                <Badge tone="neutral">{e.status}</Badge>
                <span>
                  Review <ChevronRight size={16} />
                </span>
              </button>
            ))}
          {!entries.data.pages[0].items.length && (
            <p className="muted">No entries to display.</p>
          )}
          {entries.hasNextPage && (
            <button
              className="button outline"
              onClick={() => void entries.fetchNextPage()}
            >
              Load more entries
            </button>
          )}
        </>
      )}
      {selected && (
        <ReviewDialog
          id={id}
          entry={selected}
          close={() => setSelected(null)}
        />
      )}
    </section>
  );
}
function ReviewDialog({
  id,
  entry,
  close,
}: {
  id: string;
  entry: Entry;
  close: () => void;
}) {
  const query = useQueryClient();
  const [decision, setDecision] = useState("ELIGIBLE"),
    [reason, setReason] = useState("");
  const evidence = useQuery({
    queryKey: ["creator-evidence", id, entry.id],
    queryFn: () =>
      request<EvidenceResult>(`/campaigns/${id}/entries/${entry.id}/evidence`),
  });
  return (
    <Dialog close={close}>
      <h2 id="dialog-title">Review entry #{entry.slot_number}</h2>
      <p className="mono break">{entry.payout_wallet}</p>
      {evidence.isPending ? (
        <Loading />
      ) : evidence.isError ? (
        <ErrorState error={evidence.error} />
      ) : (
        <>
          <p className="muted">
            X: {evidence.data.x_username_declared || "Not declared"} · Discord:{" "}
            {evidence.data.discord_username_declared || "Not declared"}
          </p>
          {evidence.data.evidence.map((item) => (
            <div className="evidence-preview" key={item.task_id}>
              <strong className="mono">Task {shorten(item.task_id)}</strong>
              <p>{item.evidence.text}</p>
              <SafeLink href={item.evidence.url}>Evidence link</SafeLink>
              {item.image_url && (
                <SafeLink href={item.image_url}>Open uploaded image</SafeLink>
              )}
            </div>
          ))}
        </>
      )}
      <label className="field">
        Decision
        <select value={decision} onChange={(e) => setDecision(e.target.value)}>
          <option value="ELIGIBLE">Eligible</option>
          <option value="DISQUALIFIED">Disqualified</option>
        </select>
      </label>
      <label className="field">
        Reason
        <textarea
          required
          maxLength={2000}
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          placeholder="Explain the review decision"
        />
      </label>
      <ActionButton
        disabled={!reason.trim() || evidence.isPending || evidence.isError}
        action={async () => {
          await post(`/campaigns/${id}/entries/${entry.id}/review`, {
            expected_version: entry.version,
            decision,
            reason: reason.trim(),
          });
          await query.invalidateQueries({ queryKey: ["creator-entries", id] });
          close();
        }}
      >
        Save review decision
      </ActionButton>
    </Dialog>
  );
}
function AllocationPanel({
  id,
  finalized,
}: {
  id: string;
  finalized: boolean;
}) {
  const preview = useQuery({
    queryKey: ["allocation-preview", id, finalized],
    enabled: !demo,
    queryFn: () =>
      request<AllocationPreview | Results>(
        `/campaigns/${id}/${finalized ? "results" : "allocation-preview"}`,
      ),
    refetchInterval: 15000,
  });
  return (
    <section className="panel">
      <h2>{finalized ? "Published results" : "Allocation preview"}</h2>
      {demo ? (
        <p className="muted">No allocation artifacts in demo mode.</p>
      ) : preview.isPending ? (
        <Loading />
      ) : preview.isError ? (
        <ErrorState
          error={preview.error}
          retry={() => void preview.refetch()}
        />
      ) : (
        <>
          <dl className="facts">
            <div>
              <dt>Recipients</dt>
              <dd>{preview.data.leaf_count}</dd>
            </div>
            <div>
              <dt>Merkle root</dt>
              <dd className="mono break">{preview.data.root}</dd>
            </div>
            <div>
              <dt>Manifest hash</dt>
              <dd className="mono break">{preview.data.manifest_hash}</dd>
            </div>
          </dl>
          <SafeLink href={preview.data.manifest_url}>
            Export allocation manifest
          </SafeLink>
          {"eligible_snapshot_url" in preview.data && (
            <p>
              <SafeLink href={preview.data.eligible_snapshot_url}>
                Eligibility snapshot
              </SafeLink>
            </p>
          )}
          {"raffle_transcript_url" in preview.data && (
            <SafeLink href={preview.data.raffle_transcript_url}>
              Raffle transcript
            </SafeLink>
          )}
        </>
      )}
    </section>
  );
}
