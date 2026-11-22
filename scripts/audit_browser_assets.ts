// The SSR shell embeds production CDN assets outside bun.lock. Validate the
// bytes browsers execute and look up the exact npm versions in OSV.
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";

const shell = await readFile(new URL("../crates/blog-server/src/shell.rs", import.meta.url), "utf8");
const packages = new Map<string, string>();
let assets = 0;
for (const [tag] of shell.matchAll(/<(?:script|link)\b[^>]*>/gs)) {
  const url = /\b(?:src|href)\s*=\s*["']([^"']+)["']/.exec(tag)?.[1];
  if (!url || !/^(?:[a-z][\w+.-]*:|\/\/)/i.test(url)) continue;
  // Google Fonts serves a user-agent-specific stylesheet.
  if (tag.startsWith("<link") && tag.includes('rel="stylesheet"') && url.startsWith("https://fonts.googleapis.com/css2?")) continue;
  const match = /^https:\/\/(?:cdn\.jsdelivr\.net\/npm\/([\w.-]+)@|cdnjs\.cloudflare\.com\/ajax\/libs\/([\w.-]+)\/)(\d+\.\d+\.\d+)\/[\w./-]+$/.exec(url);
  if (!match) throw new Error(`Unrecognized or unpinned browser asset: ${url}`);
  const [, jsdelivr, cdnjs, version] = match;
  const name = jsdelivr ?? cdnjs;
  const integrity = /\bintegrity="(sha384-[A-Za-z0-9+/=]+)"/.exec(tag)?.[1];
  if (!integrity || !tag.includes('crossorigin="anonymous"')) throw new Error(`Missing SRI: ${url}`);
  const response = await fetch(url, { redirect: "error", signal: AbortSignal.timeout(20000) });
  if (!response.ok) throw new Error(`${response.status} fetching ${url}`);
  const digest = createHash("sha384").update(Buffer.from(await response.arrayBuffer())).digest("base64");
  if (integrity !== `sha384-${digest}`) throw new Error(`SRI mismatch: ${url}`);
  packages.set(`${name}@${version}`, name);
  assets++;
}
if (!assets) throw new Error("No CDN assets found in the SSR shell");
const queries = Array.from(packages, ([id, name]) => ({
  package: { ecosystem: "npm", name }, version: id.slice(name.length + 1),
}));
const audit = await fetch("https://api.osv.dev/v1/querybatch", {
  method: "POST", headers: { "content-type": "application/json" },
  body: JSON.stringify({ queries }), signal: AbortSignal.timeout(30000),
});
if (!audit.ok) throw new Error(`OSV returned ${audit.status}`);
const { results } = await audit.json();
if (!Array.isArray(results) || results.length !== queries.length) throw new Error("Incomplete OSV response");
for (const [i, result] of results.entries()) {
  if (result.vulns?.length) throw new Error(`${queries[i].package.name}@${queries[i].version}: ${result.vulns.map((v: {id: string}) => v.id).join(", ")}`);
}
console.log(`Verified SRI for ${assets} assets and OSV advisories for ${Array.from(packages.keys()).join(", ")}`);
