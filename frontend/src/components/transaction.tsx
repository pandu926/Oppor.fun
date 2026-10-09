import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { ShieldCheck } from "lucide-react";
import type {
  Campaign,
  FundingResult,
  PreparedTransaction,
} from "../lib/types";
import { post } from "../lib/api";
import { demo } from "../lib/config";
import { Dialog, useApp } from "../lib/context";
import { executePrepared, shorten, validatePrepared } from "../lib/wallet";
import { ExplorerLink } from "./ui";
export function TransactionButton({
  campaign,
  kind,
  label,
  claimIndex,
}: {
  campaign: Campaign;
  kind: string;
  label: string;
  claimIndex?: number;
}) {
  const { session, connect, notify } = useApp();
  const query = useQueryClient();
  const [prepared, setPrepared] = useState<PreparedTransaction | null>(null);
  const [funding, setFunding] = useState<FundingResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [hash, setHash] = useState("");
  const [approval, setApproval] = useState(false);
  async function prepare() {
    if (!session) {
      connect();
      return;
    }
    setBusy(true);
    setError("");
    try {
      if (demo)
        throw new Error(
          "Blockchain actions are available in live mode after a contract deployment.",
        );
      const body = kind === "CLAIM" ? { claim_index: claimIndex } : undefined;
      if (kind === "FUND") {
        const result = await post<FundingResult>(
          `/campaigns/${campaign.id}/prepare-fund`,
        );
        setFunding(result);
        const tx = result.approvals[0] || result.fund;
        const needsApproval = result.approvals.length > 0;
        validatePrepared(
          tx,
          session.wallet,
          campaign,
          needsApproval ? "APPROVE" : kind,
        );
        setApproval(needsApproval);
        setPrepared(tx);
      } else {
        const tx = await post<PreparedTransaction>(
          `/campaigns/${campaign.id}/prepare-${kind.toLowerCase()}`,
          body,
        );
        validatePrepared(tx, session.wallet, campaign, kind);
        setApproval(false);
        setPrepared(tx);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Preparation failed.");
    } finally {
      setBusy(false);
    }
  }
  async function send() {
    if (!prepared || !session?.provider) return;
    setBusy(true);
    setError("");
    try {
      const receiptHash = await executePrepared(
        session.provider,
        prepared,
        session.wallet,
        campaign,
        approval ? "APPROVE" : kind,
        setHash,
      );
      setHash(receiptHash);
      setPrepared(null);
      if (!approval) {
        await post(`/campaigns/${campaign.id}/transactions`, {
          tx_hash: receiptHash,
          kind,
        }).catch(() =>
          notify(
            "Transaction confirmed. Tracking is delayed; the indexer will still process it.",
          ),
        );
        notify(
          "Transaction confirmed onchain. Waiting for indexed campaign state.",
        );
      } else notify("Approval confirmed. Prepare funding again to continue.");
      await query.invalidateQueries();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Transaction failed.");
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="action">
      <button
        className="button primary"
        disabled={busy || demo}
        onClick={() => void prepare()}
      >
        {busy ? "Waiting…" : label}
      </button>
      {demo && (
        <small className="muted">Available with a live deployment</small>
      )}
      {hash && (
        <p>
          <ExplorerLink hash={hash} />
        </p>
      )}
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {prepared && (
        <Dialog
          close={() => {
            if (!busy) setPrepared(null);
          }}
        >
          <ShieldCheck className="dialog-icon" />
          <h2 id="dialog-title">
            Review {approval ? "token approval" : kind.toLowerCase()}
          </h2>
          <p>
            Check the destination and intent before confirming in your wallet.
          </p>
          <dl className="facts">
            <div>
              <dt>Action</dt>
              <dd>
                {prepared.intent}
                {claimIndex !== undefined ? ` #${claimIndex}` : ""}
              </dd>
            </div>
            <div>
              <dt>Contract</dt>
              <dd className="mono" title={prepared.to}>
                {shorten(prepared.to)}
              </dd>
            </div>
            <div>
              <dt>Wallet</dt>
              <dd className="mono">{shorten(prepared.expected_sender)}</dd>
            </div>
            <div>
              <dt>Chain ID</dt>
              <dd>{prepared.chain_id}</dd>
            </div>
            <div>
              <dt>Native value</dt>
              <dd>{prepared.value}</dd>
            </div>
            <div>
              <dt>Valid until</dt>
              <dd>{new Date(prepared.expires_at).toLocaleTimeString()}</dd>
            </div>
          </dl>
          {funding && approval && (
            <p className="notice">
              {funding.approvals.length} approval transaction(s) remain. Confirm
              each approval, then prepare funding again. ERC-1155 approval
              grants collection-wide access to this escrow.
            </p>
          )}
          <p className="fineprint">
            The wallet displays network fees. Your position is updated after the
            backend verifies the onchain event.
          </p>
          <button
            className="button primary wide"
            disabled={busy}
            onClick={() => void send()}
          >
            {busy ? "Confirming transaction…" : "Confirm in wallet"}
          </button>
          {error && (
            <p className="error" role="alert">
              {error}
            </p>
          )}
        </Dialog>
      )}
    </div>
  );
}
