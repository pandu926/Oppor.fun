import { useEffect, useRef, useState } from "react";
import { Link, NavLink, useLocation } from "react-router-dom";
import {
  ArrowDown,
  ArrowLeft,
  ArrowRight,
  BookOpen,
  Check,
  ChevronRight,
  Code2,
  Copy,
  FileText,
  Info,
  Menu,
  Search,
  ShieldCheck,
  X,
} from "lucide-react";
import { Brand } from "../components/ui";
import { Dialog } from "../lib/context";
import {
  docGroups,
  docPages,
  docPath,
  findDoc,
  type DocPage,
  type DocSection,
} from "./content";
import "./docs.css";
export function Docs() {
  const location = useLocation();
  const page = findDoc(location.pathname);
  const [search, setSearch] = useState(false);
  const [mobile, setMobile] = useState(false);
  const [compact, setCompact] = useState(false);
  const [active, setActive] = useState(page?.sections[0]?.id || "");
  const [tocOpen, setTocOpen] = useState(false);
  const searchButton = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    const media = window.matchMedia("(max-width:760px)");
    const changed = () => setCompact(media.matches);
    changed();
    media.addEventListener("change", changed);
    return () => media.removeEventListener("change", changed);
  }, []);
  useEffect(() => {
    if (!mobile) return;
    const previous = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    document
      .querySelector<HTMLAnchorElement>(".docs-sidebar .is-active")
      ?.focus();
    const trap = (event: KeyboardEvent) => {
      if (event.key !== "Tab") return;
      const items = [
        ...document.querySelectorAll<HTMLElement>(
          ".docs-sidebar a[href],.docs-menu-button",
        ),
      ];
      const first = items[0],
        last = items.at(-1);
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last?.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first?.focus();
      }
    };
    document.addEventListener("keydown", trap);
    return () => {
      document.body.style.overflow = previous;
      document.removeEventListener("keydown", trap);
    };
  }, [mobile]);
  useEffect(() => {
    const key = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setSearch((value) => !value);
      }
      if (event.key === "Escape") {
        setMobile(false);
        document.querySelector<HTMLButtonElement>(".docs-menu-button")?.focus();
      }
    };
    document.addEventListener("keydown", key);
    return () => document.removeEventListener("keydown", key);
  }, []);
  useEffect(() => {
    setMobile(false);
    setTocOpen(false);
    setActive(page?.sections[0]?.id || "");
    const sections = Array.from(document.querySelectorAll(".doc-section"));
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries)
          if (entry.isIntersecting) setActive(entry.target.id);
      },
      { rootMargin: "-80px 0px -65% 0px" },
    );
    sections.forEach((section) => observer.observe(section));
    return () => observer.disconnect();
  }, [location.pathname, page]);
  function closeSearch() {
    setSearch(false);
    searchButton.current?.focus();
  }
  return (
    <div className="docs-shell">
      <a className="skip-link docs-skip" href="#main">
        Skip to documentation
      </a>
      <header className="docs-topbar">
        <div inert={mobile ? true : undefined} className="docs-wordmark">
          <Brand />
          <span className="docs-wordmark-divider" />
          <Link to="/docs" className="docs-label">
            Documentation
          </Link>
        </div>
        <button
          inert={mobile ? true : undefined}
          className="docs-search-trigger"
          ref={searchButton}
          onClick={() => setSearch(true)}
          aria-label="Search documentation"
        >
          <Search size={16} />
          <span>Search documentation...</span>
          <kbd>⌘ K</kbd>
        </button>
        <div className="docs-header-links">
          <Link to="/" className="docs-market-link">
            Marketplace <ArrowRight size={15} />
          </Link>
          <button
            className="docs-menu-button"
            onClick={() => setMobile((value) => !value)}
            aria-expanded={mobile}
            aria-controls="docs-navigation"
            aria-label={
              mobile ? "Close documentation menu" : "Open documentation menu"
            }
          >
            {mobile ? <X size={20} /> : <Menu size={20} />}
          </button>
        </div>
      </header>
      <div className="docs-container">
        {mobile && (
          <button
            className="docs-shade"
            aria-label="Dismiss documentation menu"
            onClick={() => setMobile(false)}
          />
        )}
        <aside
          inert={compact && !mobile ? true : undefined}
          id="docs-navigation"
          className={`docs-sidebar ${mobile ? "docs-sidebar-open" : ""}`}
        >
          <nav aria-label="Documentation">
            <Link className="docs-space" to="/docs">
              <span>
                <BookOpen size={18} />
              </span>
              <div>
                Oppor docs<small>Product &amp; developer guides</small>
              </div>
            </Link>
            {docGroups.map((group) => (
              <div className="docs-nav-group" key={group}>
                <h2>{group}</h2>
                {docPages
                  .filter((p) => p.group === group)
                  .map((p) => (
                    <NavLink
                      key={p.slug}
                      to={docPath(p.slug)}
                      end
                      onClick={() => setMobile(false)}
                      className={({ isActive }) =>
                        `docs-nav-item ${isActive ? "is-active" : ""}`
                      }
                    >
                      {p.slug === "" ? (
                        <BookOpen size={15} />
                      ) : p.slug === "api-overview" ? (
                        <Code2 size={15} />
                      ) : p.slug === "escrow-and-security" ? (
                        <ShieldCheck size={15} />
                      ) : (
                        <FileText size={15} />
                      )}
                      <span>{p.title}</span>
                    </NavLink>
                  ))}
              </div>
            ))}
          </nav>
          <div className="docs-sidebar-foot">
            <span className="docs-network-dot" /> Built for Arc
          </div>
        </aside>
        {page ? (
          <>
            <main
              id="main"
              tabIndex={-1}
              inert={mobile ? true : undefined}
              className="docs-main"
            >
              <div className="docs-breadcrumb">
                <span>{page.group}</span>
                <ChevronRight size={13} />
                <span>{page.title}</span>
              </div>
              <div className="docs-article-header">
                <span className="docs-section-label">{page.group}</span>
                <h1>{page.title}</h1>
                <p>{page.description}</p>
              </div>
              {page.slug === "" && (
                <div className="docs-overview-art" aria-hidden="true">
                  <div className="docs-art-grid" />
                  <div className="docs-art-step">
                    <span>
                      <FileText />
                    </span>
                    <b>Create a campaign</b>
                    <small>Set tasks &amp; rewards</small>
                  </div>
                  <span className="docs-art-line" />
                  <div className="docs-art-step">
                    <span>
                      <ShieldCheck />
                    </span>
                    <b>Review entries</b>
                    <small>Publish the allocation</small>
                  </div>
                  <span className="docs-art-line" />
                  <div className="docs-art-step">
                    <span>
                      <Check />
                    </span>
                    <b>Claim rewards</b>
                    <small>Direct to your wallet</small>
                  </div>
                </div>
              )}
              <button
                className="docs-mobile-toc"
                aria-expanded={tocOpen}
                onClick={() => setTocOpen((v) => !v)}
              >
                On this page <ArrowDown size={14} />
              </button>
              {tocOpen && (
                <nav className="docs-inline-toc" aria-label="Article sections">
                  {page.sections.map((section) => (
                    <a
                      href={`#${section.id}`}
                      key={section.id}
                      onClick={() => setTocOpen(false)}
                    >
                      {section.title}
                    </a>
                  ))}
                </nav>
              )}
              <article className="docs-article">
                {page.sections.map((section) => (
                  <Section key={section.id} section={section} />
                ))}
              </article>
              {page.slug === "" && (
                <div className="docs-start-grid">
                  <Link to="/docs/joining-a-campaign">
                    <span className="docs-start-icon">
                      <BookOpen size={20} />
                    </span>
                    <h3>
                      For participants <ArrowRight size={16} />
                    </h3>
                    <p>Join a campaign, submit evidence, and claim rewards.</p>
                  </Link>
                  <Link to="/docs/creating-a-campaign">
                    <span className="docs-start-icon">
                      <FileText size={20} />
                    </span>
                    <h3>
                      For creators <ArrowRight size={16} />
                    </h3>
                    <p>Configure a campaign and launch your reward pool.</p>
                  </Link>
                </div>
              )}
              <DocPagination page={page} />
              <footer className="docs-article-footer">
                <span>Oppor documentation</span>
                <Link to="/docs/faq">
                  Questions? Read the FAQ <ArrowRight size={13} />
                </Link>
              </footer>
            </main>
            <aside className="docs-toc">
              <h2>On this page</h2>
              <nav aria-label="On this page">
                {page.sections.map((section) => (
                  <a
                    key={section.id}
                    href={`#${section.id}`}
                    className={active === section.id ? "toc-active" : ""}
                    aria-current={
                      active === section.id ? "location" : undefined
                    }
                  >
                    {section.title}
                  </a>
                ))}
              </nav>
              <div className="docs-toc-help">
                <BookOpen size={17} />
                <p>New to Oppor?</p>
                <Link to="/docs/quickstart">
                  Start with the quickstart <ArrowRight size={13} />
                </Link>
              </div>
            </aside>
          </>
        ) : (
          <main id="main" className="docs-main docs-not-found" tabIndex={-1}>
            <span className="docs-section-label">Documentation</span>
            <h1>Page not found</h1>
            <p>
              This guide may have moved. Browse the documentation or search for
              a topic.
            </p>
            <Link to="/docs" className="button primary">
              Back to introduction <ArrowRight size={16} />
            </Link>
          </main>
        )}
      </div>
      {search && (
        <Dialog close={closeSearch}>
          <SearchDocs onNavigate={closeSearch} />
        </Dialog>
      )}
    </div>
  );
}
function Section({ section }: { section: DocSection }) {
  return (
    <section id={section.id} className="doc-section">
      <h2>
        <a href={`#${section.id}`}>
          {section.title}
          <span className="doc-anchor" aria-hidden="true">
            #
          </span>
        </a>
      </h2>
      {section.paragraphs?.map((text) => (
        <p key={text}>{text}</p>
      ))}
      {section.steps && (
        <ol className="doc-steps">
          {section.steps.map((step, i) => (
            <li key={step}>
              <span aria-hidden="true">{i + 1}</span>
              <p>{step}</p>
            </li>
          ))}
        </ol>
      )}
      {section.table && (
        <div className="doc-table-wrap">
          <table>
            <thead>
              <tr>
                {section.table.headers.map((header) => (
                  <th scope="col" key={header}>
                    {header}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {section.table.rows.map((row) => (
                <tr key={row[0]}>
                  {row.map((cell, i) => (
                    <td key={i}>{cell}</td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {section.code && (
        <CodeBlock language={section.code.language} text={section.code.text} />
      )}{" "}
      {section.note && (
        <aside className="doc-callout">
          <Info size={18} />
          <p>{section.note}</p>
        </aside>
      )}
    </section>
  );
}
function CodeBlock({ language, text }: { language: string; text: string }) {
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(false), 2000);
    return () => clearTimeout(timer);
  }, [copied]);
  return (
    <div className="doc-code">
      <div>
        <span>{language}</span>
        <button
          aria-label={copied ? "Copied code" : "Copy code"}
          onClick={async () => {
            try {
              await navigator.clipboard.writeText(text);
              setCopied(true);
              setError("");
            } catch {
              setError("Copy unavailable. Select the code to copy it.");
            }
          }}
        >
          {copied ? <Check size={15} /> : <Copy size={15} />}
        </button>
      </div>
      <pre>
        <code>{text}</code>
      </pre>
      {error && <p role="status">{error}</p>}
    </div>
  );
}
function DocPagination({ page }: { page: DocPage }) {
  const index = docPages.indexOf(page);
  const previous = docPages[index - 1],
    next = docPages[index + 1];
  return (
    <nav className="doc-pagination" aria-label="Previous and next articles">
      {previous ? (
        <Link to={docPath(previous.slug)}>
          <ArrowLeft size={17} />
          <div>
            <span>Previous</span>
            <strong>{previous.title}</strong>
          </div>
        </Link>
      ) : (
        <span />
      )}
      {next && (
        <Link to={docPath(next.slug)}>
          <div>
            <span>Next</span>
            <strong>{next.title}</strong>
          </div>
          <ArrowRight size={17} />
        </Link>
      )}
    </nav>
  );
}
function SearchDocs({ onNavigate }: { onNavigate: () => void }) {
  const [term, setTerm] = useState("");
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    input.current?.focus();
  }, []);
  const query = term.trim().toLowerCase();
  const words = query.split(/\s+/);
  const results = docPages
    .flatMap((page) => {
      const titleMatch = words.every((w) =>
        `${page.title} ${page.description}`.toLowerCase().includes(w),
      );
      if (!query || titleMatch)
        return [
          { page, section: null as DocSection | null, text: page.description },
        ];
      const sections = page.sections.filter((s) =>
        words.every((w) =>
          `${s.title} ${s.paragraphs?.join(" ")} ${s.steps?.join(" ")} ${s.table?.rows.flat().join(" ")} ${s.note || ""}`
            .toLowerCase()
            .includes(w),
        ),
      );
      return sections.slice(0, 2).map((section) => ({
        page,
        section,
        text: section.paragraphs?.[0] || section.note || page.description,
      }));
    })
    .slice(0, 10);
  return (
    <div className="docs-search">
      <h2 id="dialog-title">Search documentation</h2>
      <label className="docs-search-input">
        <Search size={19} />
        <input
          ref={input}
          aria-label="Search guides"
          value={term}
          onChange={(e) => setTerm(e.target.value)}
          placeholder="Search tasks, rewards, API…"
          onKeyDown={(event) => {
            if (event.key === "ArrowDown") {
              event.preventDefault();
              document
                .querySelector<HTMLAnchorElement>(".docs-search-result")
                ?.focus();
            }
          }}
        />
      </label>
      <p className="docs-search-caption" role="status">
        {query
          ? `${results.length} ${results.length === 1 ? "result" : "results"}`
          : "Popular guides"}
      </p>
      <div className="docs-search-results">
        {results.map(({ page, section, text }, i) => (
          <Link
            className="docs-search-result"
            key={`${page.slug}-${section?.id || ""}`}
            to={`${docPath(page.slug)}${section ? "#" + section.id : ""}`}
            onClick={onNavigate}
            onKeyDown={(event) => {
              const items = Array.from(
                document.querySelectorAll<HTMLAnchorElement>(
                  ".docs-search-result",
                ),
              );
              if (event.key === "ArrowDown") {
                event.preventDefault();
                items[(i + 1) % items.length]?.focus();
              }
              if (event.key === "ArrowUp") {
                event.preventDefault();
                if (i === 0) input.current?.focus();
                else items[i - 1]?.focus();
              }
            }}
          >
            <FileText size={17} />
            <div>
              <span>
                {page.group} <ChevronRight size={11} />
                {page.title}
              </span>
              <strong>{section?.title || page.title}</strong>
              <p>{text}</p>
            </div>
            <ArrowRight size={15} />
          </Link>
        ))}
      </div>
      {!results.length && (
        <p className="docs-no-results">
          No matching guides. Try “claim”, “funding”, or “evidence”.
        </p>
      )}
      <div className="docs-search-footer">
        <span>
          <kbd>↑</kbd>
          <kbd>↓</kbd> Navigate
        </span>
        <span>
          <kbd>esc</kbd> Close
        </span>
      </div>
    </div>
  );
}
