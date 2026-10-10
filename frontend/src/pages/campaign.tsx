import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowLeft,
  Check,
  Clock3,
  ExternalLink,
  LockKeyhole,
  Ticket,
} from "lucide-react";
import {
  ActionButton,
  Badge,
  DemoNotice,
  ErrorState,
  Loading,
  PageHeader,
  SafeLink,
  TokenArt,
  campaignIndex,
} from "../components/ui";
import { Reward } from "../lib/reward";
import { TransactionButton } from "../components/transaction";
import { useCampaign, useEntry } from "../lib/queries";
import { demo, secureUrl } from "../lib/config";
import { useApp } from "../lib/context";
import { post, rememberCampaign, request } from "../lib/api";
import { demoEvidence, demoRegister, demoSave, demoSubmit } from "../lib/demo";
import type {
  AllocationResult,
  Entry,
  EvidenceResult,
  TaskView,
  UploadResult,
  EvidenceMutation,
  EntryMutation,
} from "../lib/types";
export function CampaignDetail() {
  const { id = "" } = useParams();
  const c = useCampaign(id);
  const { session, connect } = useApp();
  const entry = useEntry(id);
  const query = useQueryClient();
  const [x, setX] = useState(""),
    [discord, setDiscord] = useState("");
  if (c.isPending) return <Loading />;
  if (c.isError || !c.data) return <ErrorState error={c.error} />;
  const campaign = c.data;
  const mine =
    session?.wallet.toLowerCase() === campaign.creator_wallet.toLowerCase();
  const open =
    campaign.status === "ACTIVE" && Date.parse(campaign.cutoff_at) > Date.now();
  async function register() {
    if (!session) {
      connect();
      return;
    }
    const value = demo
      ? demoRegister(id)
      : await post<Entry>(`/campaigns/${id}/entries`, {
          x_username: x || null,
          discord_username: discord || null,
        });
    rememberCampaign(session.wallet, id);
    query.setQueryData(["entry", id, session.wallet], value);
  }
  return (
    <>
      <Link className="back-link" to="/">
        <ArrowLeft size={16} />
        All campaigns
      </Link>
      <PageHeader
        title={campaign.title}
        description={campaign.description}
        action={
          mine ? (
            <Link className="button outline" to={`/campaigns/${id}/manage`}>
              Manage campaign
            </Link>
          ) : undefined
        }
      />
      <DemoNotice />
      <div className="detail-grid">
        <div>
          <section className="panel campaign-summary">
            <TokenArt index={campaignIndex(campaign)} />
            <div>
              <span className="eyebrow">Arc campaign</span>
              <h2>
                <Reward campaign={campaign} />
              </h2>
              <div className="inline">
                <Badge>
                  <Ticket size={15} />
                  {campaign.distribution.mode === "RAFFLE"
                    ? "Raffle"
                    : "All eligible"}
                </Badge>
                <Badge tone="neutral">
                  {campaign.status.replaceAll("_", " ")}
                </Badge>
              </div>
            </div>
          </section>
          <section className="panel">
            <h2>Complete the tasks</h2>
            <p className="muted">
              Submit evidence for each required task. The creator reviews
              entries after the cutoff.
            </p>
            {campaign.tasks?.map((task, index) => (
              <TaskEvidence
                key={`${task.id}-${session?.wallet || "visitor"}`}
                task={task}
                index={index}
                campaignId={id}
                entry={entry.data || null}
                editable={open && !mine}
              />
            ))}
            {entry.data && (
              <>
                <div className="notice">
                  Entry status:{" "}
                  <strong>{entry.data.status.replaceAll("_", " ")}</strong>.
                  Saving evidence requires you to submit the entry again.
                </div>
                <ActionButton
                  disabled={!open || entry.data.status === "SUBMITTED"}
                  action={async () => {
                    if (!entry.data) return;
                    const saved = demo
                      ? demoSubmit(id)
                      : await post<EntryMutation>(
                          `/campaigns/${id}/my-entry/submit`,
                          { expected_version: entry.data.version },
                        );
                    query.setQueryData(["entry", id, session?.wallet], {
                      ...entry.data,
                      ...saved,
                    });
                    await query.invalidateQueries({ queryKey: ["entry", id] });
                  }}
                >
                  Submit entry for review
                </ActionButton>
                {entry.data.review && (
                  <p className="notice">
                    Creator review: {entry.data.review.reason}
                  </p>
                )}
              </>
            )}
          </section>
          {campaign.status === "CLAIM_OPEN" && (
            <section className="panel">
              <h2>Your rewards</h2>
              <Claims id={id} />
            </section>
          )}
          <section className="panel">
            <h2>Campaign rules</h2>
            <p>
              Verification is manual. Connecting X or Discord is not required.
              Evidence links and declared usernames do not prove task
              completion.
            </p>
            <dl className="facts">
              <div>
                <dt>Distribution</dt>
                <dd>
                  {campaign.distribution.mode === "RAFFLE"
                    ? `${campaign.distribution.winner_count} raffle winners`
                    : "All eligible entries"}
                </dd>
              </div>
              <div>
                <dt>Allocation policy</dt>
                <dd>
                  {(
                    campaign.distribution.allocation_policy || "EQUAL_POOL"
                  ).replaceAll("_", " ")}
                </dd>
              </div>
              <div>
                <dt>Entry limit</dt>
                <dd>{campaign.registration_limit.toLocaleString()}</dd>
              </div>
              <div>
                <dt>Reward contract</dt>
                <dd className="mono break">{campaign.reward.token_address}</dd>
              </div>
            </dl>
            {campaign.rules && (
              <details>
                <summary>Locked campaign configuration</summary>
                <pre>{JSON.stringify(campaign.rules, null, 2)}</pre>
              </details>
            )}
            {campaign.escrow_address && (
              <p className="mono break">Escrow: {campaign.escrow_address}</p>
            )}
          </section>
        </div>
        <aside>
          <section className="panel sticky">
            <span className="eyebrow">Total reward pool</span>
            <h2 className="pool-heading">
              <Reward campaign={campaign} />
            </h2>
            <p className="muted">
              {campaign.distribution.mode === "RAFFLE"
                ? "Eligible entries enter the raffle. A reward is not guaranteed."
                : "Rewards follow the locked allocation rules."}
            </p>
            <dl className="facts timeline">
              {[
                ["Starts", campaign.start_at],
                ["Entry cutoff", campaign.cutoff_at],
                ["Review deadline", campaign.review_deadline],
                ["Claim deadline", campaign.claim_deadline],
              ].map(([label, date]) => (
                <div key={label}>
                  <dt>
                    <Clock3 size={14} />
                    {label}
                  </dt>
                  <dd>
                    {new Date(date).toLocaleString("en-US", {
                      month: "short",
                      day: "numeric",
                      hour: "2-digit",
                      minute: "2-digit",
                      timeZone: "UTC",
                    })}{" "}
                    UTC
                  </dd>
                </div>
              ))}
            </dl>
            {!mine && !entry.data && (
              <>
                <label className="field">
                  X username{" "}
                  <span className="muted">Optional, self-declared</span>
                  <input
                    value={x}
                    maxLength={100}
                    onChange={(e) => setX(e.target.value)}
                    placeholder="@yourhandle"
                  />
                </label>
                <label className="field">
                  Discord username{" "}
                  <span className="muted">Optional, self-declared</span>
                  <input
                    value={discord}
                    maxLength={100}
                    onChange={(e) => setDiscord(e.target.value)}
                    placeholder="Your username"
                  />
                </label>
                <ActionButton
                  disabled={!open || entry.isPending}
                  action={register}
                >
                  {session ? "Join campaign" : "Connect to join"}
                </ActionButton>
              </>
            )}
            {entry.isError && <ErrorState error={entry.error} />}
            <p className="fineprint">
              <LockKeyhole size={14} />
              Payout goes to your connected wallet. Review the rules before
              joining.
            </p>
          </section>
        </aside>
      </div>
    </>
  );
}
function TaskEvidence({
  task,
  index,
  campaignId,
  entry,
  editable,
}: {
  task: TaskView;
  index: number;
  campaignId: string;
  entry: Entry | null;
  editable: boolean;
}) {
  const query = useQueryClient();
  const { session } = useApp();
  const [text, setText] = useState(""),
    [url, setUrl] = useState(""),
    [file, setFile] = useState<File | null>(null),
    [saved, setSaved] = useState(false);
  const evidence = useQuery({
    queryKey: ["evidence", campaignId, entry?.id],
    enabled: !!entry,
    queryFn: () =>
      demo
        ? Promise.resolve(demoEvidence(campaignId))
        : request<EvidenceResult>(
            `/campaigns/${campaignId}/entries/${entry?.id}/evidence`,
          ),
  });
  const existing = evidence.data?.evidence.find((e) => e.task_id === task.id);
  async function save() {
    if (!entry) return;
    let uploadId: string | null = null;
    if (file) {
      if (demo)
        throw new Error(
          "Image uploads require live storage. Add text or a link in demo mode.",
        );
      if (
        !["image/png", "image/jpeg", "image/webp"].includes(file.type) ||
        file.size > 5 * 1024 * 1024 ||
        file.size === 0
      )
        throw new Error(
          "Choose a PNG, JPEG, or WebP image smaller than 5 MiB.",
        );
      const presigned = await post<UploadResult>("/uploads/presign", {
        campaign_id: campaignId,
        content_type: file.type,
        size_bytes: file.size,
      });
      if (
        presigned.upload.method !== "PUT" ||
        presigned.upload.size_bytes !== file.size
      )
        throw new Error("Invalid upload authorization.");
      const uploaded = await fetch(
        secureUrl(presigned.upload.url, import.meta.env.DEV),
        {
          method: "PUT",
          headers: { "Content-Type": file.type },
          body: file,
          credentials: "omit",
          signal: AbortSignal.timeout(60000),
        },
      );
      if (!uploaded.ok)
        throw new Error("Upload failed. Your entry has not changed.");
      await post(`/uploads/${presigned.upload_id}/complete`);
      uploadId = presigned.upload_id;
    }
    if (url) secureUrl(url);
    if (!text.trim() && !url && !uploadId)
      throw new Error("Add evidence before saving.");
    const result = demo
      ? demoSave(campaignId, task.id, text, url)
      : await request<EvidenceMutation>(
          `/campaigns/${campaignId}/my-entry/submissions/${task.id}`,
          {
            method: "PUT",
            body: {
              expected_version: entry.version,
              text: text.trim() || null,
              url: url || null,
              upload_id: uploadId,
            },
          },
        );
    query.setQueryData(["entry", campaignId, session?.wallet], {
      ...entry,
      version: result.version,
      status: "REGISTERED",
    });
    await query.invalidateQueries({ queryKey: ["evidence", campaignId] });
    setSaved(true);
  }
  return (
    <article className="task">
      <div className="task-title">
        <span className="task-number">
          {existing || saved ? <Check size={17} /> : index + 1}
        </span>
        <div>
          <h3>
            {task.task_type.replaceAll("_", " ").toLowerCase()}{" "}
            <span className="muted">
              {task.required ? "Required" : "Optional"}
            </span>
          </h3>
          <p>{task.instructions}</p>
        </div>
        <SafeLink href={task.target_url}>
          <ExternalLink size={17} />
          <span className="sr-only">Open task link</span>
        </SafeLink>
      </div>
      {existing && (
        <div className="saved-evidence">
          <p>{existing.evidence.text}</p>
          <SafeLink href={existing.evidence.url}>Saved evidence</SafeLink>
          {existing.image_url && (
            <SafeLink href={existing.image_url}>View image</SafeLink>
          )}
          <Badge>Evidence saved</Badge>
        </div>
      )}
      {entry && editable && (
        <div className="evidence-form">
          <label className="field">
            Evidence text
            <textarea
              aria-label={`Evidence for task ${index + 1}`}
              maxLength={2000}
              value={text}
              onChange={(e) => setText(e.target.value)}
              placeholder={
                existing?.evidence.text ||
                "Describe how you completed this task"
              }
            />
          </label>
          <label className="field">
            Evidence URL
            <input
              type="url"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="https://x.com/yourhandle/status/…"
            />
          </label>
          {!demo && (
            <label className="field">
              Screenshot (optional)
              <input
                type="file"
                accept="image/png,image/jpeg,image/webp"
                onChange={(e) => setFile(e.target.files?.[0] || null)}
              />
            </label>
          )}
          <ActionButton className="button outline" action={save}>
            Save evidence
          </ActionButton>
        </div>
      )}
    </article>
  );
}
export function Claims({ id }: { id: string }) {
  const { session, connect } = useApp();
  const campaign = useCampaign(id);
  const allocations = useQuery({
    queryKey: ["allocations", id, session?.wallet],
    enabled: !!session && !demo,
    queryFn: () =>
      request<AllocationResult>(
        `/campaigns/${id}/allocations/${session?.wallet}`,
      ),
  });
  if (!session)
    return (
      <button className="button primary" onClick={connect}>
        Connect to view rewards
      </button>
    );
  if (demo)
    return <p className="muted">No real allocations exist in demo mode.</p>;
  if (allocations.isPending) return <Loading />;
  if (allocations.isError) return <ErrorState error={allocations.error} />;
  function download() {
    const blob = new Blob([JSON.stringify(allocations.data, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `oppor-proof-${id}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }
  return (
    <>
      {allocations.data?.allocations.length ? (
        allocations.data.allocations.map((a) => (
          <div className="allocation-row" key={a.index}>
            <div>
              <strong>{a.quantity} base units</strong>
              <p className="muted">
                Allocation #{a.index} · Token ID {a.token_id}
              </p>
            </div>
            {a.claimed_at ? (
              <Badge>Claimed</Badge>
            ) : (
              campaign.data &&
              Number.isSafeInteger(Number(a.index)) &&
              Number(a.index) >= 0 &&
              Number(a.index) < 100000 && (
                <TransactionButton
                  campaign={campaign.data}
                  kind="CLAIM"
                  claimIndex={Number(a.index)}
                  label="Claim reward"
                />
              )
            )}
          </div>
        ))
      ) : (
        <p className="muted">No allocation for this wallet.</p>
      )}
      <button className="button outline" onClick={download}>
        Export claim proofs
      </button>
    </>
  );
}
