(function () {
  function initMermaid() {
    if (typeof mermaid === "undefined") return;

    const scheme = document.body.getAttribute("data-md-color-scheme");
    const theme = scheme === "slate" ? "dark" : "default";

    mermaid.initialize({ startOnLoad: true, theme });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initMermaid);
  } else {
    initMermaid();
  }
})();

