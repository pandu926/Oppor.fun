import { useState } from "react";
import {
  useInfiniteQuery,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import {
  ActionButton,
  AuthGate,
  Badge,
  Empty,
  ErrorState,
  Loading,
  PageHeader,
} from "../components/ui";
import { Dialog, useApp } from "../lib/context";
import { demo } from "../lib/config";
import { post, request } from "../lib/api";
import { shorten } from "../lib/wallet";
import type {
  AdminStats,
  AdminCampaignPage,
  AdminUserPage,
  AdminAuditPage,
  AdminJobPage,
  AdminCampaign,
  AdminUser,
  AdminAudit,
  AdminJob,
} from "../lib/types";
type Operation = {
  path: string;
  title: string;
  version: number;
  body: Record<string, unknown>;
};
export function Admin() {
  return (
    <>
      <PageHeader
        title="Administration"
        description="Moderate campaigns and accounts. Monitor platform activity."
      />
      <AuthGate>
        <AdminContent />
      </AuthGate>
    </>
  );
}
function AdminContent() {
  const { session, connect } = useApp();
  const [tab, setTab] = useState("campaigns");
  const [op, setOp] = useState<Operation | null>(null);
  const [reason, setReason] = useState("");
  const query = useQueryClient();
  const stats = useQuery({
    queryKey: ["admin-stats", session?.wallet],
    enabled: !!session?.admin && !demo,
    queryFn: () => request<AdminStats>("/admin/stats"),
    refetchInterval: 30000,
  });
  if (demo)
    return (
      <Empty
        title="Administration requires a live deployment"
        description="Operator access is authorized by the backend wallet allowlist. Demo accounts do not have admin permissions."
      />
    );
  if (!session?.admin)
    return (
      <Empty
        title="Restricted workspace"
        description="Connect an allowlisted operator wallet. Admin sessions require a fresh signature every 15 minutes."
      />
    );
  function campaignOp(c: AdminCampaign) {
    setReason("");
    setOp({
      path: `/admin/campaigns/${c.campaign.id}/moderation`,
      title: c.moderation.hidden ? "Restore campaign" : "Hide campaign",
      version: c.moderation.version,
      body: { hidden: !c.moderation.hidden },
    });
  }
  function userOp(u: AdminUser, revoke = false) {
    setReason("");
    setOp({
      path: `/admin/users/${u.id}/${revoke ? "revoke-sessions" : "suspension"}`,
      title: revoke
        ? "Revoke active sessions"
        : u.suspended
          ? "Restore account"
          : "Suspend account",
      version: u.version,
      body: revoke ? {} : { suspended: !u.suspended },
    });
  }
  return (
    <>
      <button className="button outline" onClick={connect}>
        Refresh admin session
      </button>
      {stats.isPending ? (
        <Loading />
      ) : stats.isError ? (
        <ErrorState error={stats.error} />
      ) : (
        <>
          <div className="metric-grid">
            {[
              ["Users", stats.data.counts.users],
              ["Campaigns", stats.data.counts.campaigns],
              ["Entries", stats.data.counts.entries],
              ["Claims", stats.data.counts.claimed_allocations],
            ].map(([label, value]) => (
              <div className="panel" key={label}>
                <p className="muted">{label}</p>
                <strong>{value}</strong>
              </div>
            ))}
          </div>
          <p className="notice">
            Indexer:{" "}
            {stats.data.indexer?.healthy
              ? "Healthy"
              : stats.data.indexer?.halted_reason || "Not reporting"}
            . Admin changes require an audit reason.
          </p>
        </>
      )}
      <div
        className="filter-tabs admin-tabs"
        role="tablist"
        aria-label="Administration section"
      >
        {["campaigns", "users", "audit-logs", "jobs"].map((t) => (
          <button
            role="tab"
            aria-selected={tab === t}
            className={tab === t ? "selected" : ""}
            key={t}
            onClick={() => setTab(t)}
          >
            {t.replaceAll("-", " ")}
          </button>
        ))}
      </div>
      <AdminTable tab={tab} campaignOp={campaignOp} userOp={userOp} />
      {op && (
        <Dialog close={() => setOp(null)}>
          <h2 id="dialog-title">{op.title}</h2>
          <p>
            Changes are recorded in the audit log. Campaign moderation does not
            rewrite onchain rewards or claim rights.
          </p>
          <label className="field">
            Reason
            <textarea
              required
              maxLength={2000}
              value={reason}
              onChange={(e) => setReason(e.target.value)}
            />
          </label>
          <ActionButton
            disabled={!reason.trim()}
            action={async () => {
              await post(op.path, {
                expected_version: op.version,
                reason: reason.trim(),
                ...op.body,
              });
              await query.invalidateQueries({ queryKey: ["admin"] });
              await query.invalidateQueries({ queryKey: ["admin-stats"] });
              setOp(null);
            }}
          >
            Confirm action
          </ActionButton>
        </Dialog>
      )}
    </>
  );
}
function AdminTable({
  tab,
  campaignOp,
  userOp,
}: {
  tab: string;
  campaignOp: (c: AdminCampaign) => void;
  userOp: (u: AdminUser, revoke?: boolean) => void;
}) {
  const [filter, setFilter] = useState("");
  const pages = useInfiniteQuery({
    queryKey: ["admin", tab, filter],
    initialPageParam: null as string | null,
    queryFn: ({ pageParam }) =>
      request<
        AdminCampaignPage | AdminUserPage | AdminAuditPage | AdminJobPage
      >(
        `/admin/${tab}?limit=25${pageParam ? `&cursor=${encodeURIComponent(pageParam)}` : ""}${filter && tab === "jobs" ? `&status=${filter}` : ""}`,
      ),
    getNextPageParam: (p) => p.next_cursor,
  });
  return (
    <section className="panel">
      {tab === "jobs" && (
        <label className="field">
          Job status
          <select value={filter} onChange={(e) => setFilter(e.target.value)}>
            <option value="">All statuses</option>
            {["PENDING", "RUNNING", "FAILED", "SUCCEEDED"].map((s) => (
              <option key={s}>{s}</option>
            ))}
          </select>
        </label>
      )}
      {pages.isPending ? (
        <Loading />
      ) : pages.isError ? (
        <ErrorState error={pages.error} retry={() => void pages.refetch()} />
      ) : (
        <>
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Record</th>
                  <th>Status / action</th>
                  <th>Details</th>
                </tr>
              </thead>
              <tbody>
                {pages.data.pages
                  .flatMap<AdminCampaign | AdminUser | AdminAudit | AdminJob>(
                    (p) => p.items,
                  )
                  .map((item) => {
                    if ("campaign" in item)
                      return (
                        <tr key={item.campaign.id}>
                          <td>
                            {item.campaign.title}
                            <p className="mono">{shorten(item.campaign.id)}</p>
                          </td>
                          <td>
                            <Badge
                              tone={item.moderation.hidden ? "rose" : "neutral"}
                            >
                              {item.moderation.hidden ? "Hidden" : "Visible"}
                            </Badge>
                          </td>
                          <td>
                            <button
                              className="button outline compact"
                              onClick={() => campaignOp(item)}
                            >
                              {item.moderation.hidden ? "Restore" : "Hide"}
                            </button>
                          </td>
                        </tr>
                      );
                    if ("wallet" in item)
                      return (
                        <tr key={item.id}>
                          <td className="mono">{shorten(item.wallet)}</td>
                          <td>
                            <Badge tone={item.suspended ? "rose" : "neutral"}>
                              {item.suspended ? "Suspended" : "Active"}
                            </Badge>
                          </td>
                          <td>
                            <div className="inline">
                              <button
                                className="button outline compact"
                                onClick={() => userOp(item)}
                              >
                                {item.suspended ? "Restore" : "Suspend"}
                              </button>
                              <button
                                className="button outline compact"
                                onClick={() => userOp(item, true)}
                              >
                                Revoke sessions
                              </button>
                            </div>
                          </td>
                        </tr>
                      );
                    if ("action" in item)
                      return (
                        <tr key={item.id}>
                          <td>
                            {item.action}
                            <p>{new Date(item.created_at).toLocaleString()}</p>
                          </td>
                          <td className="mono">
                            {item.actor_id ? shorten(item.actor_id) : "System"}
                          </td>
                          <td>
                            <details>
                              <summary>Metadata</summary>
                              <pre>
                                {JSON.stringify(item.metadata, null, 2)}
                              </pre>
                            </details>
                          </td>
                        </tr>
                      );
                    return (
                      <tr key={item.id}>
                        <td>
                          {item.kind}
                          <p className="mono">{shorten(item.campaign_id)}</p>
                        </td>
                        <td>
                          <Badge
                            tone={item.status === "FAILED" ? "rose" : "neutral"}
                          >
                            {item.status}
                          </Badge>
                        </td>
                        <td>
                          {item.attempts} attempts
                          {item.last_error && (
                            <p className="error">{item.last_error}</p>
                          )}
                        </td>
                      </tr>
                    );
                  })}
              </tbody>
            </table>
          </div>
          {!pages.data.pages[0].items.length && (
            <p className="muted">No records found.</p>
          )}
          {pages.hasNextPage && (
            <button
              className="button outline load-more"
              disabled={pages.isFetchingNextPage}
              onClick={() => void pages.fetchNextPage()}
            >
              Load more records
            </button>
          )}
        </>
      )}
    </section>
  );
}
