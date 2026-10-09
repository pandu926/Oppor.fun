import { useState, useEffect } from "react";
import { Link, NavLink, Outlet, useLocation } from "react-router-dom";
import {
  ArrowRight,
  ChartNoAxesColumn,
  ChevronDown,
  Compass,
  FileCheck2,
  Gift,
  HelpCircle,
  LogOut,
  Megaphone,
  Menu,
  Plus,
  ShieldCheck,
  Wallet,
  X,
} from "lucide-react";
import { Brand } from "./ui";
import { useApp } from "../lib/context";
import { shorten } from "../lib/wallet";
import { demo, chain } from "../lib/config";
export function Layout() {
  const [mobile, setMobile] = useState(false);
  const [compact, setCompact] = useState(false);
  useEffect(() => {
    const media = window.matchMedia("(max-width:850px)");
    const changed = () => setCompact(media.matches);
    changed();
    media.addEventListener("change", changed);
    return () => media.removeEventListener("change", changed);
  }, []);
  useEffect(() => {
    if (!mobile) return;
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setMobile(false);
        document.querySelector<HTMLButtonElement>(".menu-toggle")?.focus();
      }
    };
    document.addEventListener("keydown", key);
    document.querySelector<HTMLButtonElement>(".mobile-close")?.focus();
    return () => document.removeEventListener("keydown", key);
  }, [mobile]);
  const { session, connect, logout } = useApp();
  const location = useLocation();
  const current =
    location.pathname === "/"
      ? "Discover"
      : location.pathname.startsWith("/create")
        ? "Create campaign"
        : location.pathname.startsWith("/entries")
          ? "My entries"
          : location.pathname.startsWith("/rewards")
            ? "My rewards"
            : location.pathname === "/campaigns"
              ? "My campaigns"
              : location.pathname.startsWith("/admin")
                ? "Administration"
                : location.pathname.includes("/campaigns/")
                  ? "Campaign"
                  : location.pathname === "/how-it-works"
                    ? "How it works"
                    : "Oppor";
  return (
    <div className="app-shell">
      <a className="skip-link" href="#main">
        Skip to content
      </a>
      {mobile && (
        <button
          className="sidebar-shade"
          aria-label="Close menu"
          onClick={() => setMobile(false)}
        />
      )}
      <aside
        inert={compact && !mobile ? true : undefined}
        className={`sidebar ${mobile ? "is-open" : ""}`}
      >
        <div className="brand-row">
          <Brand />
          <button
            className="icon-button mobile-close"
            onClick={() => setMobile(false)}
            aria-label="Close menu"
          >
            <X />
          </button>
        </div>
        <p className="workspace-label">Workspace</p>
        <nav aria-label="Workspace">
          {[
            [Compass, "Discover", "/"],
            [FileCheck2, "My entries", "/entries"],
            [Gift, "My rewards", "/rewards"],
            [ChartNoAxesColumn, "My campaigns", "/campaigns"],
          ].map(([Icon, label, path]) => (
            <NavLink
              key={String(path)}
              to={String(path)}
              end
              onClick={() => setMobile(false)}
              className={({ isActive }) =>
                `nav-link ${isActive ? "active" : ""}`
              }
            >
              {typeof Icon !== "string" && <Icon size={21} />}
              <span>{String(label)}</span>
            </NavLink>
          ))}
          {session?.admin && (
            <NavLink
              to="/admin"
              className="nav-link"
              onClick={() => setMobile(false)}
            >
              <ShieldCheck size={21} />
              Admin
            </NavLink>
          )}
        </nav>
        <div className="sidebar-divider" />
        <section className="launch-panel">
          <Megaphone size={23} />
          <h3>Launch your campaign</h3>
          <p>Bring your community together</p>
          <Link
            className="button outline"
            to="/create"
            onClick={() => setMobile(false)}
          >
            Create campaign <ArrowRight size={17} />
          </Link>
        </section>
        <div className="sidebar-bottom">
          <a
            className="network-pill"
            href={chain.blockExplorers.default.url}
            target="_blank"
            rel="noopener noreferrer"
          >
            <span className="arc-icon">◉</span>Arc network{" "}
            <ChevronDown size={16} />
          </a>
          <Link
            className="help-link"
            to="/how-it-works"
            onClick={() => setMobile(false)}
          >
            <HelpCircle size={19} />
            Help &amp; support
          </Link>
        </div>
      </aside>
      <div className="workspace" inert={mobile ? true : undefined}>
        <div className="topbar">
          <div className="breadcrumbs">
            <button
              className="icon-button menu-toggle"
              aria-label="Open menu"
              aria-expanded={mobile}
              onClick={() => setMobile(true)}
            >
              <Menu />
            </button>
            <span>Marketplace</span>
            <span>/</span>
            <strong>{current}</strong>
          </div>
          <div className="topbar-actions">
            <Link to="/how-it-works" className="how-link">
              <HelpCircle size={17} />
              How it works
            </Link>
            {session ? (
              <>
                <span className="wallet-label">
                  {demo ? "Demo wallet" : shorten(session.wallet)}
                </span>
                <button
                  className="icon-button"
                  aria-label="Disconnect wallet"
                  onClick={() => void logout()}
                >
                  <LogOut size={18} />
                </button>
              </>
            ) : (
              <button
                className="button primary connect-button"
                onClick={connect}
              >
                <Wallet size={18} />
                Connect wallet
              </button>
            )}
          </div>
        </div>
        <main id="main" tabIndex={-1}>
          <Outlet />
        </main>
        <footer className="site-footer">
          <span>© {new Date().getFullYear()} Oppor</span>
          <Link to="/docs">Documentation</Link>
          <Link to="/privacy">Privacy</Link>
          <Link to="/terms">Terms</Link>
          {demo && <span>Demo environment</span>}
        </footer>
      </div>
    </div>
  );
}
export function CreateLink() {
  return (
    <Link className="button primary" to="/create">
      <Plus size={19} />
      Create campaign
    </Link>
  );
}
