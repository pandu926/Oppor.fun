import { useState } from "react";
import { Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { ArrowRight, Plus } from "lucide-react";
import {
  AuthGate,
  Badge,
  CampaignCard,
  DemoNotice,
  Empty,
  ErrorState,
  Loading,
  PageHeader,
} from "../components/ui";
import { CreateLink } from "../components/layout";
import { useCampaigns } from "../lib/queries";
import { useApp } from "../lib/context";
import { demo } from "../lib/config";
import {
  ApiError,
  rememberCampaign,
  rememberedCampaigns,
  request,
} from "../lib/api";
import { demoCampaigns, demoEntry } from "../lib/demo";
import type { Campaign, Entry } from "../lib/types";
import { Claims } from "./campaign";
export function MyCampaigns() {
  return (
    <>
      <PageHeader
        title="My campaigns"
        description="Build your community. Manage rewards, entries, and reviews."
        action={<CreateLink />}
      />
      <AuthGate>
        <DemoNotice />
        <CreatorList />
      </AuthGate>
    </>
  );
}
function CreatorList() {
  const query = useCampaigns(true, "newest");
  if (query.isPending) return <Loading />;
  if (query.isError)
    return (
      <ErrorState error={query.error} retry={() => void query.refetch()} />
    );
  const campaigns = query.data.pages.flatMap((p) => p.items);
  return (
    <>
      {campaigns.length ? (
        <div className="campaign-grid">
          {campaigns.map((c) => (
            <div key={c.id}>
              <CampaignCard campaign={c} />
              <Link className="manage-link" to={`/campaigns/${c.id}/manage`}>
                Manage campaign <ArrowRight size={15} />
              </Link>
            </div>
          ))}
        </div>
      ) : (
        <Empty
          title="Your next community starts here"
          description="Create a campaign, choose rewards, and invite your first supporters."
          action={
            <Link className="button primary" to="/create">
              <Plus size={17} />
              Create campaign
            </Link>
          }
        />
      )}{" "}
      {query.hasNextPage && (
        <button
          className="button outline load-more"
          onClick={() => void query.fetchNextPage()}
        >
          Load more
        </button>
      )}
    </>
  );
}
export function MyEntries({ rewards = false }: { rewards?: boolean }) {
  return (
    <>
      <PageHeader
        title={rewards ? "My rewards" : "My entries"}
        description={
          rewards
            ? "View your allocations and claim rewards to your wallet."
            : "Keep track of tasks, submissions, and creator reviews."
        }
      />
      <AuthGate>
        <DemoNotice />
        <EntryList rewards={rewards} />
      </AuthGate>
    </>
  );
}
function EntryList({ rewards }: { rewards: boolean }) {
  const { session } = useApp();
  const publicCampaigns = useCampaigns();
  const [importId, setImportId] = useState("");
  const [extra, setExtra] = useState<string[]>([]);
  const campaigns = publicCampaigns.data?.pages.flatMap((p) => p.items) || [];
  const query = useQuery({
    queryKey: [
      "my-entries",
      session?.wallet,
      campaigns.map((c) => c.id).join(","),
      extra,
    ],
    enabled: !!session && !publicCampaigns.isPending,
    queryFn: async () => {
      if (demo)
        return demoCampaigns()
          .map((c) => ({ campaign: c, entry: demoEntry(c.id) }))
          .filter((x) => x.entry) as { campaign: Campaign; entry: Entry }[];
      const ids = [
        ...new Set([
          ...rememberedCampaigns(session!.wallet),
          ...campaigns.map((c) => c.id),
          ...extra,
        ]),
      ];
      const result: { campaign: Campaign; entry: Entry }[] = [];
      for (let i = 0; i < ids.length; i += 4) {
        await Promise.all(
          ids.slice(i, i + 4).map(async (id) => {
            try {
              const entry = await request<Entry>(`/campaigns/${id}/my-entry`);
              const campaign =
                campaigns.find((c) => c.id === id) ||
                (await request<Campaign>(`/campaigns/${id}`));
              result.push({ entry, campaign });
            } catch (e) {
              if (!(e instanceof ApiError && e.status === 404)) throw e;
            }
          }),
        );
      }
      return result;
    },
  });
  return (
    <>
      {!demo && (
        <p className="notice">
          Entries from saved and currently loaded public campaigns. Load more
          campaigns or add a campaign ID to recover entries from another device.
        </p>
      )}
      {query.isPending ? (
        <Loading />
      ) : query.isError ? (
        <ErrorState error={query.error} retry={() => void query.refetch()} />
      ) : !query.data?.length ? (
        <Empty
          title={
            rewards
              ? "No rewards to claim yet"
              : "Your first campaign is waiting"
          }
          description={
            rewards
              ? "Eligible entries receive rewards after creator review and finalization."
              : "Discover a community, complete its tasks, and submit your evidence."
          }
          action={
            <Link className="button primary" to="/">
              Discover campaigns <ArrowRight size={17} />
            </Link>
          }
        />
      ) : (
        <div className="entry-list">
          {query.data.map(({ entry, campaign }) => (
            <section key={entry.id} className="panel">
              <div className="section-heading">
                <div>
                  <Link className="title-link" to={`/campaigns/${campaign.id}`}>
                    {campaign.title}
                  </Link>
                  <p className="muted">Entry #{entry.slot_number}</p>
                </div>
                <Badge
                  tone={entry.status === "DISQUALIFIED" ? "rose" : "indigo"}
                >
                  {entry.status.replaceAll("_", " ")}
                </Badge>
              </div>
              {entry.review && <p>{entry.review.reason}</p>}
              {rewards ? (
                <Claims id={campaign.id} />
              ) : (
                <Link
                  className="button outline"
                  to={`/campaigns/${campaign.id}`}
                >
                  {entry.status === "REGISTERED"
                    ? "Continue tasks"
                    : "View entry"}
                  <ArrowRight size={16} />
                </Link>
              )}
            </section>
          ))}
        </div>
      )}
      {!demo && (
        <>
          <form
            className="import-form"
            onSubmit={(e) => {
              e.preventDefault();
              if (!/^[0-9a-f-]{36}$/i.test(importId)) return;
              rememberCampaign(session!.wallet, importId);
              setExtra([...extra, importId]);
              setImportId("");
            }}
          >
            <label className="field">
              Recover by campaign ID
              <input
                required
                pattern="[0-9a-fA-F-]{36}"
                value={importId}
                onChange={(e) => setImportId(e.target.value)}
                placeholder="Campaign UUID"
              />
            </label>
            <button className="button outline">Add campaign</button>
          </form>
          {publicCampaigns.hasNextPage && (
            <button
              className="button outline"
              onClick={() => void publicCampaigns.fetchNextPage()}
            >
              Search more public campaigns
            </button>
          )}
        </>
      )}
    </>
  );
}
