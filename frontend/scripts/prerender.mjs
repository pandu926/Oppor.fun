import { readFile, writeFile, mkdir } from "node:fs/promises";
import { resolve } from "node:path";
import { render, docPages } from "../dist-server/entry-server.js";
const template = await readFile("dist/index.html", "utf8");
const escape = (value) =>
  String(value)
    .replaceAll("&", "&amp;")
    .replaceAll('"', "&quot;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
let campaigns = [];
if (process.env.PRERENDER_API_URL) {
  const base = new URL(process.env.PRERENDER_API_URL);
  if (
    base.protocol !== "https:" ||
    base.username ||
    base.password ||
    base.search ||
    base.hash
  )
    throw new Error("PRERENDER_API_URL must use HTTPS.");
  const response = await fetch(
    base.href.replace(/\/$/, "") + "/campaigns?sort=ending&limit=100",
    { signal: AbortSignal.timeout(15000) },
  );
  if (!response.ok)
    throw new Error("Public campaign snapshot could not be loaded.");
  campaigns = (await response.json()).items;
  if (
    !Array.isArray(campaigns) ||
    campaigns.length > 100 ||
    campaigns.some(
      (c) =>
        !/^([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})$/i.test(
          c.id,
        ),
    )
  )
    throw new Error("Invalid public campaign snapshot.");
  for (let i = 0; i < campaigns.length; i++) {
    const detail = await fetch(
      base.href.replace(/\/$/, "") + `/campaigns/${campaigns[i].id}`,
      { signal: AbortSignal.timeout(15000) },
    );
    if (!detail.ok)
      throw new Error("Public campaign detail could not be loaded.");
    campaigns[i] = await detail.json();
  }
}
const routes = [
  "/",
  "/how-it-works",
  "/privacy",
  "/terms",
  "/create",
  "/entries",
  "/rewards",
  "/campaigns",
  "/admin",
  ...docPages.map((page) => (page.slug ? `/docs/${page.slug}` : "/docs")),
  ...campaigns.map((c) => `/campaigns/${c.id}`),
];
let canonical = "https://oppor.fun";
for (const path of routes) {
  const { html, state, meta } = render(path, campaigns);
  canonical = new URL(meta.canonical).origin;
  const head = `<title>${escape(meta.title)}</title><meta name="description" content="${escape(meta.description)}"><meta name="robots" content="${escape(meta.robots)}"><link rel="canonical" href="${escape(meta.canonical)}"><meta property="og:type" content="website"><meta property="og:title" content="${escape(meta.title)}"><meta property="og:description" content="${escape(meta.description)}"><meta property="og:url" content="${escape(meta.canonical)}"><meta property="og:image" content="${canonical}/social-card.png"><meta name="twitter:card" content="summary_large_image">`;
  const structured = JSON.stringify({
    "@context": "https://schema.org",
    "@type": path === "/" ? "WebSite" : "WebPage",
    name: meta.title,
    url: meta.canonical,
    description: meta.description,
  }).replaceAll("<", "\\u003c");
  const data = JSON.stringify({ path, state })
    .replaceAll("<", "\\u003c")
    .replaceAll("\u2028", "\\u2028")
    .replaceAll("\u2029", "\\u2029");
  const output = template
    .replace(
      "<!--seo-->",
      head + `<script type="application/ld+json">${structured}</script>`,
    )
    .replace("<!--app-->", html)
    .replace("<!--data-->", `<script>window.__OPPOR__=${data}</script>`);
  const folder = resolve("dist", path.slice(1));
  await mkdir(folder, { recursive: true });
  await writeFile(resolve(folder, "index.html"), output);
}
await writeFile(
  "dist/200.html",
  template.replace(
    "<!--seo-->",
    '<title>Oppor</title><meta name="robots" content="noindex,follow">',
  ),
);
const sitemap = routes.filter(
  (p) =>
    ["/", "/how-it-works", "/privacy", "/terms"].includes(p) ||
    p === "/docs" ||
    p.startsWith("/docs/") ||
    /^\/campaigns\/[^/]+$/.test(p),
);
const live = render("/").meta.robots.startsWith("index");
await writeFile(
  "dist/_redirects",
  docPages
    .map((page) => {
      const route = page.slug ? `/docs/${page.slug}` : "/docs";
      return `${route} ${route}/index.html 200`;
    })
    .join("\n") + "\n/* /200.html 200\n",
);
await writeFile(
  "dist/sitemap.xml",
  `<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">${live ? sitemap.map((p) => `<url><loc>${escape(canonical + p)}</loc></url>`).join("") : ""}</urlset>`,
);
await writeFile(
  "dist/robots.txt",
  `User-agent: *\n${live ? "Allow: /\nDisallow: /admin\nDisallow: /create\nDisallow: /entries\nDisallow: /rewards" : "Disallow: /"}\nSitemap: ${canonical}/sitemap.xml\n`,
);
console.log(`Prerendered ${routes.length} routes (${live ? "live" : "demo"}).`);
