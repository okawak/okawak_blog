// Opening, light dismissal, Escape, focus restoration, and accessibility state
// belong to the native popover. Only document transitions need coordination.
(() => {
  const closeMenus = () => {
    document.querySelectorAll('[data-mobile-navigation]:popover-open')
      .forEach((menu) => menu.hidePopover());
  };
  // Delegation survives Topcoat's document morphs; no per-render listeners.
  document.addEventListener('click', (event) => {
    if (event.target instanceof Element &&
        event.target.closest('[data-mobile-navigation] a[href]')) {
      closeMenus();
    }
  }, true);
  window.addEventListener('popstate', closeMenus);
  window.addEventListener('pagehide', closeMenus);
  const desktop = window.matchMedia('(min-width: 48rem)');
  desktop.addEventListener('change', () => {
    if (desktop.matches) closeMenus();
  });
})();
