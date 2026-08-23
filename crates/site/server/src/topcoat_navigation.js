// Topcoat 0.6.2 does not ship a client router. Keep this compatibility layer small and
// replace it when Topcoat provides first-class navigation.
const PAGE_METADATA_SELECTOR = [
  "title",
  'meta[name="description"]',
  'link[rel="canonical"]',
  'meta[property^="og:"]',
  'meta[property^="article:"]',
].join(",");

let activeNavigation;

function eligibleAnchor(event) {
  if (
    event.defaultPrevented ||
    event.button !== 0 ||
    event.metaKey ||
    event.ctrlKey ||
    event.shiftKey ||
    event.altKey
  ) {
    return null;
  }

  if (!(event.target instanceof Element)) return null;

  const anchor = event.target.closest("a[href]");
  if (
    !anchor ||
    anchor.hasAttribute("download") ||
    (anchor.target && anchor.target !== "_self")
  ) {
    return null;
  }

  const url = new URL(anchor.href, window.location.href);
  if (
    url.origin !== window.location.origin ||
    (url.pathname === window.location.pathname &&
      url.search === window.location.search &&
      url.hash)
  ) {
    return null;
  }

  return url;
}

function replacePageMetadata(nextDocument) {
  document.head
    .querySelectorAll(PAGE_METADATA_SELECTOR)
    .forEach((element) => element.remove());
  nextDocument.head
    .querySelectorAll(PAGE_METADATA_SELECTOR)
    .forEach((element) => document.head.append(document.importNode(element, true)));
}

function syncHeaderNavigation(nextDocument) {
  const nextLinks = new Map(
    Array.from(nextDocument.querySelectorAll("#site-header-nav a[href]"), (link) => [
      link.getAttribute("href"),
      link,
    ]),
  );

  document.querySelectorAll("#site-header-nav a[href]").forEach((link) => {
    const nextLink = nextLinks.get(link.getAttribute("href"));
    if (!nextLink) return;

    link.className = nextLink.className;
    if (nextLink.hasAttribute("aria-current")) {
      link.setAttribute("aria-current", nextLink.getAttribute("aria-current"));
    } else {
      link.removeAttribute("aria-current");
    }
  });
}

function initializeGeneratedContent(root) {
  window.okawakScheduleMathRender?.(root);
  window.okawakScheduleCodeHighlight?.(root);
}

function fallBackToDocumentNavigation(url) {
  window.location.assign(url.href);
}

async function navigate(url, { history = "push", scroll = true } = {}) {
  activeNavigation?.abort();
  const controller = new AbortController();
  activeNavigation = controller;
  document.querySelector("main")?.setAttribute("aria-busy", "true");

  try {
    const response = await fetch(url, {
      credentials: "same-origin",
      headers: { "X-Okawak-Navigation": "1" },
      signal: controller.signal,
    });
    const contentType = response.headers.get("content-type") ?? "";
    if (!contentType.toLowerCase().startsWith("text/html")) {
      fallBackToDocumentNavigation(url);
      return;
    }

    const nextDocument = new DOMParser().parseFromString(
      await response.text(),
      "text/html",
    );
    const currentMain = document.querySelector("main");
    const nextMain = nextDocument.querySelector("main");
    if (!currentMain || !nextMain || !nextDocument.title) {
      fallBackToDocumentNavigation(url);
      return;
    }

    const destination = new URL(response.url || url.href, window.location.href);
    replacePageMetadata(nextDocument);
    syncHeaderNavigation(nextDocument);
    const importedMain = document.importNode(nextMain, true);
    currentMain.replaceWith(importedMain);
    document.documentElement.lang = nextDocument.documentElement.lang || "ja";

    if (history === "push") {
      window.history.pushState(null, "", destination);
    }
    if (scroll) window.scrollTo(0, 0);
    initializeGeneratedContent(importedMain);
    document.dispatchEvent(
      new CustomEvent("okawak:navigation", {
        detail: { url: destination.href },
      }),
    );
  } catch (error) {
    if (error?.name !== "AbortError") fallBackToDocumentNavigation(url);
  } finally {
    if (activeNavigation === controller) activeNavigation = undefined;
    document.querySelector("main")?.removeAttribute("aria-busy");
  }
}

document.addEventListener("click", (event) => {
  const url = eligibleAnchor(event);
  if (!url) return;

  event.preventDefault();
  void navigate(url);
});

window.addEventListener("popstate", () => {
  void navigate(new URL(window.location.href), { history: "none", scroll: false });
});
