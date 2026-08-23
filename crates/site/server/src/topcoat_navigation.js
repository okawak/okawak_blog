// Topcoat 0.6.2 does not ship a client router. Keep this compatibility layer small and
// replace it when Topcoat provides first-class navigation.
const PAGE_METADATA_SELECTOR = [
  "title",
  'meta[name="description"]',
  'link[rel="canonical"]',
  'meta[property^="og:"]',
  'meta[property^="article:"]',
].join(",");
const SHELL_RUNTIME_SELECTOR = ['link[rel="stylesheet"][href]', "script"].join(
  ",",
);
const SHELL_VERSION_SELECTOR = 'meta[name="okawak-shell-version"]';
const SCROLL_STATE_KEY = "okawakScrollPosition";

let activeNavigation;
let renderedLocation = new URL(window.location.href);
let scrollFrame;

function historyStateWithScroll(position) {
  const state =
    window.history.state && typeof window.history.state === "object"
      ? window.history.state
      : {};
  return { ...state, [SCROLL_STATE_KEY]: position };
}

function currentScrollPosition() {
  return { x: window.scrollX, y: window.scrollY };
}

function savedScrollPosition(state) {
  const position = state?.[SCROLL_STATE_KEY];
  if (
    !position ||
    !Number.isFinite(position.x) ||
    !Number.isFinite(position.y)
  ) {
    return null;
  }
  return position;
}

function rememberCurrentScrollPosition() {
  window.history.replaceState(
    historyStateWithScroll(currentScrollPosition()),
    "",
    window.location.href,
  );
}

function samePage(left, right) {
  return left.pathname === right.pathname && left.search === right.search;
}

function fragmentNavigation(anchor) {
  if (!anchor) return null;

  const href = anchor.getAttribute("href") ?? "";
  const browserLocation = new URL(window.location.href);
  if (href.startsWith("#")) {
    return {
      browserLocation,
      destination: new URL(href, renderedLocation.href),
    };
  }

  const destination = new URL(anchor.href, browserLocation.href);
  if (
    destination.origin === browserLocation.origin &&
    destination.hash &&
    samePage(destination, browserLocation)
  ) {
    return { browserLocation, destination };
  }
  return null;
}

function handlesCurrentTab(event, anchor) {
  if (
    event.defaultPrevented ||
    event.button !== 0 ||
    event.metaKey ||
    event.ctrlKey ||
    event.shiftKey ||
    event.altKey ||
    !anchor ||
    anchor.hasAttribute("download") ||
    (anchor.target && anchor.target !== "_self")
  ) {
    return false;
  }
  return true;
}

function eligibleAnchor(event) {
  if (!(event.target instanceof Element)) return null;

  const anchor = event.target.closest("a[href]");
  const href = anchor?.getAttribute("href") ?? "";
  if (!handlesCurrentTab(event, anchor) || href.startsWith("#")) {
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

function shellRuntimeSignature(root, baseUrl) {
  return Array.from(root.head?.querySelectorAll(SHELL_RUNTIME_SELECTOR) ?? []).map(
    (element) => {
      if (element.tagName === "LINK") {
        return JSON.stringify([
          "LINK",
          new URL(element.getAttribute("href"), baseUrl).href,
          element.getAttribute("media") ?? "",
          element.getAttribute("integrity") ?? "",
          element.getAttribute("crossorigin") ?? "",
        ]);
      }

      const source = element.getAttribute("src");
      return JSON.stringify([
        "SCRIPT",
        source ? new URL(source, baseUrl).href : "",
        element.getAttribute("type") ?? "",
        element.hasAttribute("defer"),
        element.hasAttribute("async"),
        element.getAttribute("integrity") ?? "",
        element.getAttribute("crossorigin") ?? "",
        element.getAttribute("onload") ?? "",
        element.textContent ?? "",
      ]);
    },
  );
}

function signaturesMatch(current, next) {
  return (
    current.length === next.length &&
    current.every((value, index) => value === next[index])
  );
}

function shellMatches(nextDocument, destination) {
  const current = shellRuntimeSignature(document, window.location.href);
  const next = shellRuntimeSignature(nextDocument, destination.href);
  const currentVersion = document.head.querySelector(
    SHELL_VERSION_SELECTOR,
  )?.content;
  const nextVersion = nextDocument.head.querySelector(
    SHELL_VERSION_SELECTOR,
  )?.content;
  return (
    currentVersion &&
    currentVersion === nextVersion &&
    signaturesMatch(current, next) &&
    next.length > 0
  );
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

function scrollToDestination(destination) {
  if (!destination.hash) {
    window.scrollTo(0, 0);
    return null;
  }

  let fragment = destination.hash.slice(1);
  try {
    fragment = decodeURIComponent(fragment);
  } catch {
    // Keep the encoded fragment when it contains a malformed escape sequence.
  }

  const target =
    document.getElementById(fragment) ?? document.getElementsByName(fragment)[0];
  if (target) {
    target.scrollIntoView();
    return target;
  } else {
    window.scrollTo(0, 0);
    return null;
  }
}

function focusNavigationTarget(main, fragmentTarget) {
  const target = fragmentTarget ?? main;
  const needsTabIndex = target.tabIndex < 0 && !target.hasAttribute("tabindex");
  if (needsTabIndex) target.setAttribute("tabindex", "-1");
  target.focus({ preventScroll: true });
  if (needsTabIndex) {
    target.addEventListener("blur", () => target.removeAttribute("tabindex"), {
      once: true,
    });
  }
}

function fallBackToDocumentNavigation(url) {
  window.location.assign(url.href);
}

async function navigate(url, { history = "push", scroll = "destination" } = {}) {
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
    const destination = new URL(response.url || url.href, window.location.href);
    if (url.hash && !destination.hash) destination.hash = url.hash;
    const currentMain = document.querySelector("main");
    const nextMain = nextDocument.querySelector("main");
    if (!currentMain || !nextMain || !nextDocument.title) {
      fallBackToDocumentNavigation(url);
      return;
    }
    if (!shellMatches(nextDocument, destination)) {
      fallBackToDocumentNavigation(destination);
      return;
    }

    replacePageMetadata(nextDocument);
    syncHeaderNavigation(nextDocument);
    const importedMain = document.importNode(nextMain, true);
    currentMain.replaceWith(importedMain);
    document.documentElement.lang = nextDocument.documentElement.lang || "ja";
    renderedLocation = destination;

    if (history === "push") {
      window.history.pushState(
        historyStateWithScroll({ x: 0, y: 0 }),
        "",
        destination,
      );
    }
    initializeGeneratedContent(importedMain);
    let fragmentTarget = null;
    if (scroll === "destination") {
      fragmentTarget = scrollToDestination(destination);
    } else if (scroll) {
      window.scrollTo(scroll.x, scroll.y);
    }
    focusNavigationTarget(importedMain, fragmentTarget);
    document.dispatchEvent(
      new CustomEvent("okawak:navigation", {
        detail: { url: destination.href },
      }),
    );
  } catch (error) {
    if (error?.name !== "AbortError") fallBackToDocumentNavigation(url);
  } finally {
    if (activeNavigation === controller) {
      activeNavigation = undefined;
      document.querySelector("main")?.removeAttribute("aria-busy");
    }
  }
}

document.addEventListener("click", (event) => {
  const anchor =
    event.target instanceof Element ? event.target.closest("a[href]") : null;
  const fragment = handlesCurrentTab(event, anchor)
    ? fragmentNavigation(anchor)
    : null;
  if (fragment) {
    activeNavigation?.abort();
    if (!samePage(fragment.browserLocation, renderedLocation)) {
      event.preventDefault();
      fallBackToDocumentNavigation(fragment.destination);
      return;
    }
    rememberCurrentScrollPosition();
    return;
  }

  const url = eligibleAnchor(event);
  if (!url) return;

  event.preventDefault();
  rememberCurrentScrollPosition();
  void navigate(url);
});

window.addEventListener("popstate", (event) => {
  const destination = new URL(window.location.href);
  if (
    destination.pathname === renderedLocation.pathname &&
    destination.search === renderedLocation.search
  ) {
    activeNavigation?.abort();
    renderedLocation = destination;
    const position = savedScrollPosition(event.state);
    if (position) {
      window.scrollTo(position.x, position.y);
    } else {
      scrollToDestination(destination);
    }
    return;
  }

  void navigate(destination, {
    history: "none",
    scroll: savedScrollPosition(event.state) ?? { x: 0, y: 0 },
  });
});

if ("scrollRestoration" in window.history) {
  window.history.scrollRestoration = "manual";
}
rememberCurrentScrollPosition();
window.addEventListener(
  "scroll",
  () => {
    if (scrollFrame) return;
    scrollFrame = window.requestAnimationFrame(() => {
      scrollFrame = undefined;
      rememberCurrentScrollPosition();
    });
  },
  { passive: true },
);
