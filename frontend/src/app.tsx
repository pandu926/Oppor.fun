import { Component, useEffect, type ReactNode } from "react";
import { Route, Routes, useLocation } from "react-router-dom";
import { Docs } from "./docs/docs";
import { findDoc } from "./docs/content";
import { Layout } from "./components/layout";
import { ErrorState } from "./components/ui";
import { Discover } from "./pages/discover";
import { CampaignDetail } from "./pages/campaign";
import { Create } from "./pages/create";
import { Manage } from "./pages/manage";
import { MyCampaigns, MyEntries } from "./pages/workspace";
import { Admin } from "./pages/admin";
import { HowItWorks, Legal, NotFound } from "./pages/info";
import { useQuery, skipToken } from "@tanstack/react-query";
import type { Campaign } from "./lib/types";
import { useApp } from "./lib/context";
import { demo, siteUrl } from "./lib/config";
export function seo(path: string, campaign?: Campaign) {
  const doc = findDoc(path);
  const title =
    doc?.title ||
    campaign?.title ||
    (path === "/"
      ? "Discover campaigns"
      : path === "/how-it-works"
        ? "How Oppor works"
        : path === "/privacy"
          ? "Privacy"
          : path === "/terms"
            ? "Terms"
            : path === "/create"
              ? "Create a campaign"
              : path === "/entries"
                ? "My entries"
                : path === "/rewards"
                  ? "My rewards"
                  : path === "/admin"
                    ? "Administration"
                    : path === "/campaigns"
                      ? "My campaigns"
                      : "Campaign");
  const indexable =
    ["/", "/how-it-works", "/privacy", "/terms"].includes(path) ||
    !!campaign ||
    !!doc;
  return {
    title: `${title} · Oppor`,
    description:
      doc?.description ||
      campaign?.description ||
      "Discover community campaigns on Arc. Complete tasks, submit evidence, and claim creator-funded rewards.",
    canonical:
      siteUrl.replace(/\/$/, "") + (doc ? path.replace(/\/$/, "") : path),
    robots: !demo && indexable ? "index,follow" : "noindex,follow",
  };
}
function ensureMeta(name: string) {
  let node = document.querySelector(`meta[name="${name}"]`);
  if (!node) {
    node = document.createElement("meta");
    node.setAttribute("name", name);
    document.head.appendChild(node);
  }
  return node;
}
function PageMeta() {
  const location = useLocation();
  const { session } = useApp();
  const id = /^\/campaigns\/([^/]+)$/.exec(location.pathname)?.[1];
  const campaign = useQuery<Campaign>({
    queryKey: ["campaign", id],
    queryFn: skipToken,
  });
  useEffect(() => {
    const meta = seo(location.pathname, !session ? campaign.data : undefined);
    document.title = meta.title;
    ensureMeta("description")?.setAttribute("content", meta.description);
    ensureMeta("robots")?.setAttribute("content", meta.robots);
    document
      .querySelector('link[rel="canonical"]')
      ?.setAttribute("href", meta.canonical);
    document
      .querySelector('meta[property="og:title"]')
      ?.setAttribute("content", meta.title);
    document
      .querySelector('meta[property="og:url"]')
      ?.setAttribute("content", meta.canonical);
    document.getElementById("main")?.focus({ preventScroll: true });
    window.scrollTo(0, 0);
  }, [location.pathname, campaign.data, session]);
  useEffect(() => {
    if (location.hash) {
      const id = decodeURIComponent(location.hash.slice(1));
      requestAnimationFrame(() =>
        document.getElementById(id)?.scrollIntoView({ block: "start" }),
      );
    }
  }, [location.pathname, location.hash]);
  return null;
}
export class Boundary extends Component<
  { children: ReactNode },
  { error: Error | null }
> {
  state = { error: null as Error | null };
  static getDerivedStateFromError(error: Error) {
    return { error };
  }
  render() {
    return this.state.error ? (
      <ErrorState
        error={this.state.error}
        retry={() => window.location.reload()}
      />
    ) : (
      this.props.children
    );
  }
}
export function App() {
  return (
    <Boundary>
      <PageMeta />
      <Routes>
        <Route path="docs/*" element={<Docs />} />
        <Route element={<Layout />}>
          <Route index element={<Discover />} />
          <Route path="campaigns/:id" element={<CampaignDetail />} />
          <Route path="campaigns/:id/manage" element={<Manage />} />
          <Route path="create" element={<Create />} />
          <Route path="campaigns" element={<MyCampaigns />} />
          <Route path="entries" element={<MyEntries />} />
          <Route path="rewards" element={<MyEntries rewards />} />
          <Route path="admin" element={<Admin />} />
          <Route path="how-it-works" element={<HowItWorks />} />
          <Route path="privacy" element={<Legal privacy />} />
          <Route path="terms" element={<Legal />} />
          <Route path="*" element={<NotFound />} />
        </Route>
      </Routes>
    </Boundary>
  );
}
