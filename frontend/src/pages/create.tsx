import { useState, type FormEvent } from "react";
import { useNavigate } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import { isAddress } from "viem";
import { Plus, Trash2 } from "lucide-react";
import { AuthGate, DemoNotice, PageHeader } from "../components/ui";
import { demo, chain } from "../lib/config";
import { useApp } from "../lib/context";
import { post, request } from "../lib/api";
import { createDemo } from "../lib/demo";
import { uint256 } from "../lib/wallet";
import type { Campaign, CampaignInput, Task } from "../lib/types";
function dateInput(value: string) {
  const date = new Date(value);
  return new Date(date.getTime() - date.getTimezoneOffset() * 60000)
    .toISOString()
    .slice(0, 16);
}
const later = (days: number) =>
  dateInput(new Date(Date.now() + days * 86400000).toISOString());
export function Create() {
  return (
    <>
      <PageHeader
        title="Create a campaign"
        description="Bring your community together. Choose your rewards and set clear rules."
      />
      <AuthGate>
        <DemoNotice />
        <CampaignForm />
      </AuthGate>
    </>
  );
}
export function CampaignForm({
  initial,
  onSaved,
}: {
  initial?: Campaign;
  onSaved?: () => void;
}) {
  const { session, notify } = useApp();
  const navigate = useNavigate();
  const query = useQueryClient();
  const [title, setTitle] = useState(initial?.title || ""),
    [description, setDescription] = useState(initial?.description || ""),
    [asset, setAsset] = useState(initial?.reward.asset_kind || "ERC20"),
    [token, setToken] = useState(initial?.reward.token_address || ""),
    [amount, setAmount] = useState(initial?.reward.amount_base_units || ""),
    [tokenId, setTokenId] = useState(initial?.reward.token_id || "0"),
    [inventory, setInventory] = useState(
      initial?.reward.nft_inventory?.join(", ") || "",
    ),
    [mode, setMode] = useState(initial?.distribution.mode || "RAFFLE"),
    [policy, setPolicy] = useState(
      initial?.distribution.allocation_policy || "EQUAL_POOL",
    ),
    [winners, setWinners] = useState(initial?.distribution.winner_count || 100),
    [capacity, setCapacity] = useState(initial?.distribution.capacity || 0),
    [limit, setLimit] = useState(
      initial?.distribution.registration_limit || 10000,
    ),
    [perReward, setPerReward] = useState(
      initial?.distribution.reward_per_recipient || "",
    ),
    [refund, setRefund] = useState(""),
    [start, setStart] = useState(
      initial ? dateInput(initial.start_at) : later(1),
    ),
    [cutoff, setCutoff] = useState(
      initial ? dateInput(initial.cutoff_at) : later(8),
    ),
    [review, setReview] = useState(
      initial ? dateInput(initial.review_deadline) : later(10),
    ),
    [claim, setClaim] = useState(
      initial ? dateInput(initial.claim_deadline) : later(17),
    );
  const [tasks, setTasks] = useState<Task[]>(
    initial?.tasks?.map(
      ({ task_type, target_url, instructions, required }) => ({
        task_type,
        target_url,
        instructions,
        required,
      }),
    ) || [
      {
        task_type: "X_REPOST",
        target_url: "",
        instructions: "",
        required: true,
      },
    ],
  );
  const [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const [retryKey, setRetryKey] = useState<string | null>(null);
  const [retryPayload, setRetryPayload] = useState<string | null>(null);
  async function submit(e: FormEvent) {
    e.preventDefault();
    setError("");
    setBusy(true);
    try {
      if (!session) throw new Error("Connect your wallet first.");
      if (!isAddress(token) || /^0x0{40}$/i.test(token))
        throw new Error("Enter the reward token contract address.");
      if (refund && !isAddress(refund))
        throw new Error("Enter a valid refund wallet.");
      const ids =
        asset === "ERC721"
          ? inventory
              .split(",")
              .map((x) => x.trim())
              .filter(Boolean)
          : [];
      ids.forEach(uint256);
      if (
        new Set(ids).size !== ids.length ||
        ids.length > 100 ||
        (asset === "ERC721" && !ids.length)
      )
        throw new Error("Provide 1–100 unique NFT token IDs.");
      const total = asset === "ERC721" ? String(ids.length) : uint256(amount);
      if (BigInt(total) === 0n)
        throw new Error("Reward amount must be positive.");
      if (
        !Number.isInteger(winners) ||
        winners < 1 ||
        !Number.isInteger(limit) ||
        limit < 1 ||
        limit > 100000 ||
        !Number.isInteger(capacity) ||
        capacity < 0 ||
        capacity > 100000
      )
        throw new Error("Check winner count, capacity, and entry limit.");
      if (policy === "FIXED_REWARD") {
        uint256(perReward);
        if (
          capacity <= 0 ||
          BigInt(perReward) === 0n ||
          BigInt(perReward) * BigInt(capacity) !== BigInt(total)
        )
          throw new Error("Fixed reward × capacity must equal the total pool.");
      }
      const dates = [start, cutoff, review, claim].map((d) =>
        new Date(d).getTime(),
      );
      if (
        dates.some((d) => !Number.isFinite(d)) ||
        dates.some((d, i) => i > 0 && d <= dates[i - 1]) ||
        dates[0] <= Date.now()
      )
        throw new Error(
          "Choose future dates in order: start, cutoff, review, claim.",
        );
      if (mode === "RAFFLE" && winners > (capacity || limit))
        throw new Error("Winners cannot exceed the effective entry limit.");
      if (dates[3] - dates[2] < 86400000)
        throw new Error("Allow at least 24 hours for claims after review.");
      if (capacity > limit)
        throw new Error("Capacity cannot exceed the entry limit.");
      if (!tasks.some((t) => t.required))
        throw new Error("At least one task must be required.");
      if (
        asset === "ERC721" &&
        (policy === "FIXED_REWARD" ||
          (mode === "RAFFLE"
            ? winners > ids.length
            : capacity === 0 || capacity > ids.length))
      )
        throw new Error(
          "NFT inventory must cover winners or capacity. Fixed rewards require divisible tokens.",
        );
      for (const t of tasks) {
        if (!(t.instructions || "").trim())
          throw new Error("Each task needs clear instructions.");
        const url = new URL(t.target_url);
        if (url.protocol !== "https:" || url.username || url.password)
          throw new Error("Task links must use HTTPS without credentials.");
        if (
          t.task_type.startsWith("X_") &&
          !["x.com", "www.x.com", "twitter.com", "www.twitter.com"].includes(
            url.hostname,
          )
        )
          throw new Error("X tasks must link to X or Twitter.");
        if (
          t.task_type === "DISCORD_JOIN" &&
          !["discord.gg", "discord.com", "www.discord.com"].includes(
            url.hostname,
          )
        )
          throw new Error("Discord tasks must link to Discord.");
      }
      const input: CampaignInput = {
        title: title.trim(),
        description: description.trim(),
        chain_id: String(chain.id),
        reward: {
          asset_kind: asset,
          token_address: token,
          amount_base_units: total,
          token_id: asset === "ERC1155" ? uint256(tokenId) : "0",
          nft_inventory: ids,
        },
        distribution: {
          mode,
          winner_count: mode === "RAFFLE" ? winners : 0,
          capacity,
          registration_limit: limit,
          allocation_policy: policy,
          reward_per_recipient: policy === "FIXED_REWARD" ? perReward : null,
        },
        start_at: new Date(start).toISOString().replace(".000Z", "Z"),
        cutoff_at: new Date(cutoff).toISOString().replace(".000Z", "Z"),
        review_deadline: new Date(review).toISOString().replace(".000Z", "Z"),
        claim_deadline: new Date(claim).toISOString().replace(".000Z", "Z"),
        refund_recipient: refund || null,
        tasks: tasks.map((t) => ({ ...t, target_url: t.target_url || "" })),
      };
      const body = initial
        ? { expected_version: initial.version, campaign: input }
        : input;
      const serialized = JSON.stringify(body);
      const key =
        retryPayload === serialized && retryKey
          ? retryKey
          : crypto.randomUUID();
      setRetryKey(key);
      setRetryPayload(serialized);
      let result: Campaign;
      if (demo) {
        if (initial)
          throw new Error(
            "Draft editing is available in live mode. Create another sample draft instead.",
          );
        result = createDemo(input);
      } else
        result = initial
          ? await request<Campaign>(`/campaigns/${initial.id}`, {
              method: "PATCH",
              body,
              key,
            })
          : await post<Campaign>("/campaigns", body, key);
      await query.invalidateQueries();
      notify(
        initial
          ? "Draft updated."
          : "Campaign draft created. Fund and activate it when you are ready.",
      );
      if (onSaved) onSaved();
      else navigate(`/campaigns/${result.id}/manage`);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Could not save the draft.");
    } finally {
      setBusy(false);
    }
  }
  function updateTask(index: number, key: keyof Task, value: string | boolean) {
    setTasks(tasks.map((t, i) => (i === index ? { ...t, [key]: value } : t)));
  }
  return (
    <form onSubmit={(e) => void submit(e)} className="campaign-form">
      <section className="panel">
        <span className="eyebrow">01 · Campaign details</span>
        <h2>Make a good first impression</h2>
        <label className="field">
          Campaign name
          <input
            required
            minLength={3}
            maxLength={100}
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="Your community launch"
          />
        </label>
        <label className="field">
          Description
          <textarea
            maxLength={5000}
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="Tell people about your community and what they can earn."
          />
        </label>
      </section>
      <section className="panel">
        <span className="eyebrow">02 · Rewards &amp; distribution</span>
        <h2>Choose your reward pool</h2>
        <div className="form-grid">
          <label className="field">
            Reward asset
            <select
              value={asset}
              onChange={(e) => setAsset(e.target.value as typeof asset)}
            >
              <option value="ERC20">Token · ERC-20</option>
              <option value="ERC721">NFT · ERC-721</option>
              <option value="ERC1155">Edition · ERC-1155</option>
            </select>
          </label>
          <label className="field">
            Token contract address
            <input
              required
              value={token}
              onChange={(e) => setToken(e.target.value)}
              placeholder="0x…"
            />
          </label>
          {asset === "ERC721" ? (
            <label className="field full">
              NFT token IDs (comma-separated)
              <textarea
                required
                value={inventory}
                onChange={(e) => setInventory(e.target.value)}
                placeholder="1, 2, 3"
              />
            </label>
          ) : (
            <label className="field">
              Total reward in base units
              <input
                required
                inputMode="numeric"
                pattern="[0-9]+"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                placeholder="2500000000"
              />
              <small>
                Use the token’s exact decimals. For a token with 6 decimals, 1
                token = 1,000,000 base units.
              </small>
            </label>
          )}
          {asset === "ERC1155" && (
            <label className="field">
              Token ID
              <input
                required
                inputMode="numeric"
                value={tokenId}
                onChange={(e) => setTokenId(e.target.value)}
              />
            </label>
          )}
          <label className="field">
            Distribution
            <select
              value={mode}
              onChange={(e) => setMode(e.target.value as typeof mode)}
            >
              <option value="RAFFLE">Raffle</option>
              <option value="ALL_ELIGIBLE">All eligible</option>
            </select>
          </label>
          {mode === "RAFFLE" && (
            <label className="field">
              Number of winners
              <input
                type="number"
                min={1}
                max={100000}
                value={winners}
                onChange={(e) => setWinners(Number(e.target.value))}
              />
            </label>
          )}
          <label className="field">
            Allocation policy
            <select
              value={policy}
              onChange={(e) => setPolicy(e.target.value as typeof policy)}
            >
              <option value="EQUAL_POOL">Split pool equally</option>
              <option value="FIXED_REWARD">Fixed reward per recipient</option>
            </select>
          </label>
          <label className="field">
            Entry limit
            <input
              type="number"
              required
              min={1}
              max={100000}
              value={limit}
              onChange={(e) => setLimit(Number(e.target.value))}
            />
          </label>
          <label className="field">
            Capacity (0 uses entry limit)
            <input
              type="number"
              min={0}
              max={100000}
              value={capacity}
              onChange={(e) => setCapacity(Number(e.target.value))}
            />
          </label>
          {policy === "FIXED_REWARD" && (
            <label className="field">
              Reward per recipient in base units
              <input
                required
                inputMode="numeric"
                value={perReward}
                onChange={(e) => setPerReward(e.target.value)}
              />
            </label>
          )}
          <label className="field">
            Refund wallet (optional)
            <input
              value={refund}
              onChange={(e) => setRefund(e.target.value)}
              placeholder="Defaults to your wallet"
            />
          </label>
        </div>
        <p className="notice">
          One reward asset per campaign. The reward pool is escrowed before
          activation. Campaign configuration becomes immutable when locked.
        </p>
      </section>
      <section className="panel">
        <span className="eyebrow">03 · Schedule</span>
        <h2>Set clear deadlines</h2>
        <p className="muted">
          Dates below use your local timezone; they are saved in UTC.
        </p>
        <div className="form-grid">
          {[
            ["Start", start, setStart],
            ["Entry cutoff", cutoff, setCutoff],
            ["Review deadline", review, setReview],
            ["Claim deadline", claim, setClaim],
          ].map(([label, value, setter]) => (
            <label className="field" key={String(label)}>
              {String(label)}
              <input
                type="datetime-local"
                required
                value={String(value)}
                onChange={(e) =>
                  (setter as (v: string) => void)(e.target.value)
                }
              />
            </label>
          ))}
        </div>
      </section>
      <section className="panel">
        <span className="eyebrow">04 · Community tasks</span>
        <h2>Give people a reason to participate</h2>
        {tasks.map((task, index) => (
          <div className="task-editor" key={index}>
            <div className="section-heading">
              <h3>Task {index + 1}</h3>
              <button
                type="button"
                className="icon-button"
                aria-label={`Remove task ${index + 1}`}
                disabled={tasks.length === 1}
                onClick={() => setTasks(tasks.filter((_, i) => i !== index))}
              >
                <Trash2 size={17} />
              </button>
            </div>
            <div className="form-grid">
              <label className="field">
                Task type
                <select
                  value={task.task_type}
                  onChange={(e) =>
                    updateTask(index, "task_type", e.target.value)
                  }
                >
                  {[
                    "X_REPOST",
                    "X_LIKE",
                    "X_COMMENT",
                    "X_TAG",
                    "DISCORD_JOIN",
                    "CUSTOM",
                  ].map((t) => (
                    <option key={t}>{t}</option>
                  ))}
                </select>
              </label>
              <label className="field">
                Target URL
                <input
                  type="url"
                  required
                  value={task.target_url || ""}
                  onChange={(e) =>
                    updateTask(index, "target_url", e.target.value)
                  }
                  placeholder="https://…"
                />
              </label>
              <label className="field full">
                Instructions
                <textarea
                  required
                  maxLength={2000}
                  value={task.instructions}
                  onChange={(e) =>
                    updateTask(index, "instructions", e.target.value)
                  }
                  placeholder="Explain the task and the evidence you expect."
                />
              </label>
            </div>
            <label className="checkbox">
              <input
                type="checkbox"
                checked={task.required}
                onChange={(e) =>
                  updateTask(index, "required", e.target.checked)
                }
              />
              Required for submission
            </label>
          </div>
        ))}
        <button
          type="button"
          className="button outline"
          disabled={tasks.length >= 20}
          onClick={() =>
            setTasks([
              ...tasks,
              {
                task_type: "CUSTOM",
                target_url: "",
                instructions: "",
                required: true,
              },
            ])
          }
        >
          <Plus size={17} />
          Add task
        </button>
        <p className="fineprint">
          Social tasks are reviewed by the creator. Oppor does not automatically
          verify likes, reposts, or Discord membership.
        </p>
      </section>
      <div className="form-footer">
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        <button className="button primary" type="submit" disabled={busy}>
          {busy
            ? "Saving draft…"
            : initial
              ? "Save draft changes"
              : "Create campaign draft"}
        </button>
        <p className="muted">
          A draft is private until deployed, funded, and activated.
        </p>
      </div>
    </form>
  );
}
