import { useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import {
  ArrowRight,
  ChevronDown,
  LockKeyhole,
  Search,
  Ticket,
} from "lucide-react";
import {
  Badge,
  CampaignCard,
  Empty,
  ErrorState,
  FlowStrip,
  Loading,
  OrbitArt,
  PageHeader,
} from "../components/ui";
import { CreateLink } from "../components/layout";
import { useCampaigns } from "../lib/queries";
import { demo, usdcAddress } from "../lib/config";
import { Reward } from "../lib/reward";
import { samples } from "../lib/demo";
export function Discover() {
  const [params, setParams] = useSearchParams();
  const filter = params.get("asset") || "all";
  const term = params.get("q") || "";
  const [sort, setSort] = useState("ending");
  const campaigns = useCampaigns(false, sort);
  const items = campaigns.data?.pages.flatMap((p) => p.items) || [];
  const shown = items.filter(
    (c) =>
      (filter === "all" ||
        (filter === "usdc" &&
          (demo
            ? [samples[0].id, samples[3].id].includes(c.id)
            : c.reward.token_address.toLowerCase() ===
              usdcAddress.toLowerCase())) ||
        (filter === "tokens" &&
          c.reward.asset_kind === "ERC20" &&
          (demo
            ? ![samples[0].id, samples[3].id].includes(c.id)
            : c.reward.token_address.toLowerCase() !==
              usdcAddress.toLowerCase())) ||
        (filter === "nfts" && c.reward.asset_kind !== "ERC20")) &&
      `${c.title} ${c.description}`.toLowerCase().includes(term.toLowerCase()),
  );
  const featured = items.find((c) => c.status === "ACTIVE") || items[0];
  function change(key: string, value: string) {
    const next = new URLSearchParams(params);
    if (value && value !== "all") next.set(key, value);
    else next.delete(key);
    setParams(next, { replace: true });
  }
  return (
    <>
      <PageHeader
        title="Discover campaigns"
        description="Find communities worth joining. Earn rewards for taking part."
        action={<CreateLink />}
      />
      {featured && (
        <section className="featured">
          <div className="featured-content">
            <span className="eyebrow">Featured campaign</span>
            <h2>
              {demo && featured.id === samples[0].id
                ? "The Orbit community launch"
                : featured.title}
            </h2>
            <p>
              {demo && featured.id === samples[0].id
                ? "Help a new community find its first supporters."
                : featured.description}
            </p>
            <Link className="button outline" to={`/campaigns/${featured.id}`}>
              View campaign <ArrowRight size={17} />
            </Link>
          </div>
          <OrbitArt />
          <div className="featured-reward">
            <strong>
              <Reward campaign={featured} />
            </strong>
            <p>Total reward pool</p>
            <div className="inline">
              <Badge>
                <Ticket size={15} />
                {featured.distribution.mode === "RAFFLE"
                  ? "Raffle"
                  : "All eligible"}
              </Badge>
              {featured.status === "ACTIVE" && (
                <Badge>
                  <LockKeyhole size={15} />
                  Reward funded
                </Badge>
              )}
            </div>
          </div>
        </section>
      )}
      <div className="filter-bar">
        <div className="filter-tabs" role="tablist" aria-label="Reward asset">
          {[
            ["all", "All campaigns"],
            ["usdc", "USDC"],
            ["tokens", "Tokens"],
            ["nfts", "NFTs"],
          ].map(([key, label]) => (
            <button
              key={key}
              role="tab"
              aria-selected={filter === key}
              className={filter === key ? "selected" : ""}
              onClick={() => change("asset", key)}
            >
              {label}
            </button>
          ))}
        </div>
        <label className="search-field">
          <Search size={19} />
          <input
            type="search"
            aria-label="Search campaigns"
            placeholder="Search campaigns"
            value={term}
            onChange={(e) => change("q", e.target.value)}
          />
        </label>
        <label className="sort-field">
          <span className="sr-only">Sort campaigns</span>
          <select value={sort} onChange={(e) => setSort(e.target.value)}>
            <option value="ending">Ending soon</option>
            <option value="newest">Newest first</option>
          </select>
          <ChevronDown size={16} />
        </label>
      </div>
      {campaigns.isPending ? (
        <Loading />
      ) : campaigns.isError ? (
        <ErrorState
          error={campaigns.error}
          retry={() => void campaigns.refetch()}
        />
      ) : (
        <>
          <p className="result-count">
            Showing {shown.length}{" "}
            {shown.length === 1 ? "campaign" : "campaigns"}
            {term && ` matching “${term}”`}
          </p>
          <div className="campaign-grid">
            {shown.map((c) => (
              <CampaignCard key={c.id} campaign={c} />
            ))}
          </div>
          {!shown.length && (
            <Empty
              title="No campaigns found"
              description="Try a different search or reward type."
              action={
                <button
                  className="button outline"
                  onClick={() => setParams({})}
                >
                  Clear filters
                </button>
              }
            />
          )}{" "}
          {campaigns.hasNextPage && (
            <button
              className="button outline load-more"
              disabled={campaigns.isFetchingNextPage}
              onClick={() => void campaigns.fetchNextPage()}
            >
              Load more campaigns
            </button>
          )}
        </>
      )}
      <FlowStrip />
    </>
  );
}
