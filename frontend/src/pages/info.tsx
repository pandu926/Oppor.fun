import { Link } from "react-router-dom";
import {
  ArrowRight,
  FileCheck2,
  Gift,
  ListTodo,
  ShieldCheck,
} from "lucide-react";
import { Empty, PageHeader } from "../components/ui";
export function HowItWorks() {
  return (
    <>
      <PageHeader
        title="Communities grow together"
        description="Clear tasks. Creator review. Rewards claimed to your wallet."
        action={
          <Link className="button primary" to="/">
            Find a campaign <ArrowRight size={17} />
          </Link>
        }
      />
      <div className="info-grid">
        {[
          [
            ListTodo,
            "Find your community",
            "Compare reward assets, distribution rules, tasks, and deadlines. Join with the wallet you want to receive rewards.",
          ],
          [
            FileCheck2,
            "Complete and submit",
            "Complete each task, save evidence, then submit your entry. Editing evidence requires a new submission.",
          ],
          [
            ShieldCheck,
            "Wait for creator review",
            "After cutoff, the creator reviews evidence. Submitted entries can be eligible or disqualified. Social activity is not automatically verified.",
          ],
          [
            Gift,
            "Claim your rewards",
            "Once the reward allocation is finalized onchain, eligible recipients can claim before the deadline. Raffle entry does not guarantee a reward.",
          ],
        ].map(([Icon, title, text]) => (
          <section className="panel" key={String(title)}>
            {typeof Icon !== "string" && <Icon className="info-icon" />}
            <h2>{String(title)}</h2>
            <p>{String(text)}</p>
          </section>
        ))}
      </div>
      <section className="panel prose">
        <h2>For creators</h2>
        <p>
          Create a private draft with an ERC-20, ERC-721, or ERC-1155 reward.
          Choose a raffle or rewards for all eligible entries. Lock the
          configuration, deploy the escrow, fund the full pool, and activate the
          campaign.
        </p>
        <p>
          Review every submitted entry before the review deadline. Lock
          eligibility, inspect the allocation manifest, and finalize the
          distribution. Participants claim directly from the escrow. Unclaimed
          rewards follow the contract’s refund rules.
        </p>
        <h2>What does “reward funded” mean?</h2>
        <p>
          The backend has recognized activation with the required funding. It
          does not guarantee eligibility, raffle selection, token value, or the
          creator’s review decisions. Read the campaign’s locked rules.
        </p>
        <h2>Can I verify rewards myself?</h2>
        <p>
          Public results expose eligibility, allocation hashes, and the manifest
          after confirmed finalization. Download your claim proofs for a direct
          contract claim if the interface becomes unavailable.
        </p>
        <h2>Need help?</h2>
        <p>
          Check your wallet’s network, the campaign deadlines, and your
          submission status. A transaction receipt can precede an indexed UI
          update. Refresh the campaign after confirmation.
        </p>
        <Link className="button outline" to="/create">
          Launch a campaign <ArrowRight size={17} />
        </Link>
      </section>
    </>
  );
}
export function Legal({ privacy = false }: { privacy?: boolean }) {
  return (
    <>
      <PageHeader
        title={privacy ? "Privacy information" : "Platform terms"}
        description="Understand the information you share and the rules of participation."
      />
      <article className="panel prose">
        {privacy ? (
          <>
            <h2>Wallet and session information</h2>
            <p>
              Signing in associates your public wallet address with a session.
              Session cookies are used for authentication. The browser keeps the
              session’s request token in memory and remembers public campaign
              IDs locally to help you recover your entries.
            </p>
            <h2>Campaigns and evidence</h2>
            <p>
              Campaign information, escrow activity, and finalized reward
              results are public. Evidence, uploaded images, and declared social
              usernames are accessible to the submitting participant and
              campaign creator through authenticated requests. Do not upload
              sensitive personal information.
            </p>
            <h2>External services</h2>
            <p>
              Wallet providers, blockchain RPC services, explorers, and task
              links are external services with their own policies. Oppor does
              not use an X API to verify social activity.
            </p>
            <h2>Demo data</h2>
            <p>
              Demo drafts and entries are stored only in your browser. Clearing
              site storage removes them. Demo wallets are illustrative and
              cannot receive real rewards.
            </p>
          </>
        ) : (
          <>
            <h2>Campaign participation</h2>
            <p>
              Each campaign defines its reward asset, task requirements,
              distribution policy, and deadlines. Creators review entries
              manually. An entry must be explicitly submitted; saving evidence
              alone is insufficient.
            </p>
            <h2>Rewards and claims</h2>
            <p>
              Raffle participation does not guarantee selection. Eligibility
              does not guarantee a token’s market value. Claim only from the
              configured campaign escrow and before its claim deadline. Your
              wallet may require gas fees.
            </p>
            <h2>Creator responsibilities</h2>
            <p>
              Creators must describe tasks and rewards accurately, fund the
              campaign before activation, and review evidence within the
              configured period. Locked configurations cannot be edited.
            </p>
            <h2>Account and campaign moderation</h2>
            <p>
              Operators can hide campaign listings or suspend accounts. These
              actions do not rewrite onchain escrow balances or existing claim
              rights.
            </p>
            <h2>Deployment status</h2>
            <p>
              The default frontend is a demonstration. Live operations require a
              configured backend and deployed contracts. Internal security
              review is not an independent third-party audit.
            </p>
          </>
        )}
      </article>
    </>
  );
}
export function NotFound() {
  return (
    <Empty
      title="This page has wandered off"
      description="The page may have moved, or this campaign is no longer available."
      action={
        <Link className="button primary" to="/">
          Back to Discover
        </Link>
      }
    />
  );
}
