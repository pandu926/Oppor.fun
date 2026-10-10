import {
  mkdir,
  cp,
  symlink,
  rename,
  readFile,
  writeFile,
} from "node:fs/promises";
import { execFileSync } from "node:child_process";
const source = new URL("../frontend/dist/", import.meta.url);
const html = await readFile(new URL("index.html", source), "utf8");
if (!html.includes("oppor.fun") || !html.includes('id="root"'))
  throw new Error("The frontend build is missing Oppor canonical metadata.");
const root = "/var/lib/oppor/web";
const commit = execFileSync("git", ["rev-parse", "--short", "HEAD"], {
  cwd: new URL("../", import.meta.url),
  encoding: "utf8",
}).trim();
const release = new Date().toISOString().replace(/[^0-9]/g, "") + "-" + commit;
await mkdir(root + "/releases", { recursive: true });
await mkdir(root + "/assets", { recursive: true });
await cp(source, root + "/releases/" + release, {
  recursive: true,
  errorOnExist: true,
  force: false,
});
// Keep hashes from earlier releases available during in-flight browser requests.
await cp(new URL("assets/", source), root + "/assets", { recursive: true });
await writeFile(
  root + "/releases/" + release + "/release.json",
  JSON.stringify({ commit, published_at: new Date().toISOString() }, null, 2) +
    "\n",
);
const next = root + "/.current-" + release;
await symlink("releases/" + release, next);
await rename(next, root + "/current");
console.log("Published frontend atomically:", release);
