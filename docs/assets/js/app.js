/**
 * PortProbe Documentation Application Entry Point
 * Manages hash routing, document loading, landing page rendering,
 * section scrolling, and UI component hydration.
 */

(function () {
  const docsData = window.PORTPROBE_DOCS || {};
  let currentPath = '';

  function parseHash(hashStr) {
    if (!hashStr || hashStr === '#' || hashStr === '#/') {
      return { docPath: 'README.md', sectionId: '' };
    }

    // Remove leading #
    let clean = hashStr.startsWith('#') ? hashStr.slice(1) : hashStr;

    // If clean starts with '/' strip it
    if (clean.startsWith('/')) {
      clean = clean.slice(1);
    }

    // Check if it contains a section anchor: e.g. "getting-started/quick-start.md#expected-terminal-output"
    let docPart = clean;
    let sectionPart = '';
    const secondHashIdx = clean.indexOf('#');
    if (secondHashIdx !== -1) {
      docPart = clean.slice(0, secondHashIdx);
      sectionPart = clean.slice(secondHashIdx + 1);
    }

    // If clean has no '/' and does not end with '.md', and we already have a loaded currentPath:
    // It is a bare in-page section anchor (e.g. "#expected-terminal-output")
    if (!docPart.includes('/') && !docPart.endsWith('.md') && currentPath && !docsData[docPart]) {
      return { docPath: currentPath, sectionId: docPart };
    }

    // Default to README.md if docPart is empty
    if (!docPart || docPart === '') {
      docPart = 'README.md';
    }

    return { docPath: docPart, sectionId: sectionPart };
  }

  function scrollToSection(sectionId) {
    if (!sectionId) return;
    const targetEl = document.getElementById(sectionId);
    if (targetEl) {
      const yOffset = -70; // Header height offset
      const y = targetEl.getBoundingClientRect().top + window.pageYOffset + yOffset;
      window.scrollTo({ top: y, behavior: 'smooth' });
    }
  }

  function init() {
    PortProbeRenderer.init();
    PortProbeNav.init(window.PORTPROBE_NAVIGATION);
    PortProbeSearch.init(window.PORTPROBE_SEARCH_INDEX);

    // Setup mobile drawer toggle
    const menuBtn = document.getElementById('mobile-menu-btn');
    const closeBtn = document.getElementById('sidebar-close-btn');
    const sidebar = document.getElementById('app-sidebar');
    const backdrop = document.getElementById('sidebar-backdrop');

    function openDrawer() {
      if (sidebar) sidebar.classList.add('drawer-open');
      if (backdrop) backdrop.classList.add('open');
      document.body.classList.add('drawer-open');
    }

    function closeDrawer() {
      if (sidebar) sidebar.classList.remove('drawer-open');
      if (backdrop) backdrop.classList.remove('open');
      document.body.classList.remove('drawer-open');
    }

    if (menuBtn) {
      menuBtn.addEventListener('click', () => {
        if (sidebar && sidebar.classList.contains('drawer-open')) {
          closeDrawer();
        } else {
          openDrawer();
        }
      });
    }

    if (closeBtn) {
      closeBtn.addEventListener('click', closeDrawer);
    }

    if (backdrop) {
      backdrop.addEventListener('click', closeDrawer);
    }

    // Hash change routing
    window.addEventListener('hashchange', handleRoute);
    handleRoute();
  }

  function handleRoute() {
    const hash = window.location.hash || '#/README.md';
    const { docPath, sectionId } = parseHash(hash);

    // Close mobile drawer on route transition
    const sidebar = document.getElementById('app-sidebar');
    const backdrop = document.getElementById('sidebar-backdrop');
    if (sidebar && backdrop) {
      sidebar.classList.remove('drawer-open');
      backdrop.classList.remove('open');
      document.body.classList.remove('drawer-open');
    }

    // If staying on the same document, just scroll to section without re-rendering
    const contentArea = document.getElementById('markdown-content');
    if (docPath === currentPath && contentArea && contentArea.children.length > 0) {
      if (sectionId) {
        scrollToSection(sectionId);
      } else {
        window.scrollTo({ top: 0, behavior: 'smooth' });
      }
      return;
    }

    currentPath = docPath;
    loadDocument(docPath, sectionId);
  }

  function loadDocument(path, sectionId) {
    const contentArea = document.getElementById('markdown-content');
    const metaArea = document.getElementById('doc-meta-container');
    const landingArea = document.getElementById('landing-hero-container');
    if (!contentArea) return;

    const doc = docsData[path];

    if (path === 'README.md' && landingArea) {
      renderLandingHero(landingArea);
    } else if (landingArea) {
      landingArea.innerHTML = '';
    }

    if (doc) {
      // Update browser window title
      document.title = `${doc.title} — PortProbe Documentation`;

      // Render Metadata Card if metadata exists
      if (doc.metadata && Object.keys(doc.metadata).length > 0 && path !== 'README.md') {
        renderMetaCard(metaArea, doc.metadata);
      } else if (metaArea) {
        metaArea.innerHTML = '';
      }

      // Render Markdown
      contentArea.innerHTML = PortProbeRenderer.render(doc.markdown, doc.path);
      PortProbeRenderer.postprocessHtml(contentArea, doc.path);
      PortProbeRenderer.renderMermaid();

      // Generate Table of Contents with doc.path
      PortProbeToc.generate(doc.headings, doc.path);

      // Update Navigation Active State
      PortProbeNav.setActive(doc.path);

      // Scroll to specific section if requested, otherwise top of page
      if (sectionId) {
        setTimeout(() => {
          scrollToSection(sectionId);
        }, 80);
      } else {
        window.scrollTo({ top: 0, behavior: 'instant' });
      }
    } else {
      renderNotFound(path, contentArea);
    }
  }

  function renderLandingHero(container) {
    container.innerHTML = `
      <section class="landing-hero">
        <div class="hero-pill">
          <span>PORTPROBE V2.0.0 RELEASED</span>
          <span>•</span>
          <span>ENTERPRISE TCP PROBE</span>
        </div>
        <h1 class="hero-title">
          Asynchronous High-Concurrency<br/>
          <span class="gradient-text">TCP Connectivity Verification</span>
        </h1>
        <p class="hero-desc">
          Validate network reachability, verify firewall rules, test load balancer health probes, and diagnose distributed infrastructure paths with a lightweight, secure Rust daemon.
        </p>
        <div class="hero-actions">
          <a href="#/getting-started/quick-start.md" class="btn-primary">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polygon points="10 8 16 12 10 16 10 8"/></svg>
            Quick-Start Guide
          </a>
          <a href="#/product/architecture.md" class="btn-secondary">
            Explore Architecture
          </a>
          <a href="#/reference/cli-reference.md" class="btn-secondary">
            CLI Reference
          </a>
        </div>
        <div class="hero-snippet">
          <code>$ portprobe --health-check --bind 10.0.0.50 --port 8080</code>
          <button class="copy-btn" onclick="navigator.clipboard.writeText('portprobe --health-check --bind 10.0.0.50 --port 8080')">
            <span class="copy-text">Copy</span>
          </button>
        </div>
      </section>

      <section>
        <h2 style="font-size: 1.5rem; font-weight: 700; text-align: center; margin-bottom: 0.5rem;">Built for Modern SRE & Network Engineering</h2>
        <p style="text-align: center; color: var(--color-text-secondary); max-width: 600px; margin: 0 auto 2rem auto;">
          High-performance network diagnostics without deploying multi-tier enterprise workloads.
        </p>
        <div class="capabilities-grid">
          <div class="capability-card">
            <div class="card-icon">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 12h-4l-3 9L9 3l-3 9H2"/></svg>
            </div>
            <div class="card-title">Multi-Port & Range Probing</div>
            <div class="card-desc">Bind up to 50 listening sockets or continuous ranges (<code>80,443,8080-8085</code>) concurrently on IPv4 and IPv6 interfaces.</div>
          </div>

          <div class="capability-card">
            <div class="card-icon">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>
            </div>
            <div class="card-title">Pure-Rust TLS 1.3 & mTLS</div>
            <div class="card-desc">Native asynchronous encryption powered by <code>rustls</code> with mutual TLS, X.509 expiration alerting, and strict banking compliance.</div>
          </div>

          <div class="capability-card">
            <div class="card-icon">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10"/></svg>
            </div>
            <div class="card-title">Admission Control Pipeline</div>
            <div class="card-desc">Token-bucket rate limiters (global & per-IP), IPv6 <code>/64</code> aggregation, semaphore concurrency caps, and CIDR ingress filtering.</div>
          </div>

          <div class="capability-card">
            <div class="card-icon">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="m10 15 5-3-5-3v6Z"/></svg>
            </div>
            <div class="card-title">Client Health Check Mode</div>
            <div class="card-desc">Active client prober with <code>--health-check</code>, banner validation, and deterministic exit codes for CI/CD gates.</div>
          </div>

          <div class="capability-card">
            <div class="card-icon">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 14 14"/></svg>
            </div>
            <div class="card-title">Anti-Slowloris Architecture</div>
            <div class="card-desc">Immediate socket reclamation post-greeting banner by default, completely eliminating connection slot exhaustion.</div>
          </div>

          <div class="capability-card">
            <div class="card-icon">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 3v18h18"/><path d="m19 9-5 5-4-4-3 3"/></svg>
            </div>
            <div class="card-title">Prometheus Telemetry</div>
            <div class="card-desc">Dedicated admin HTTP server with <code>/metrics</code> exposition and constant-time Bearer token authentication.</div>
          </div>
        </div>
      </section>
    `;
  }

  function renderMetaCard(container, metadata) {
    if (!container) return;
    let html = '<div class="doc-meta-card">';
    for (const [key, val] of Object.entries(metadata)) {
      html += `
        <div class="meta-item">
          <span class="meta-label">${key}:</span>
          <span class="meta-value">${val}</span>
        </div>
      `;
    }
    html += '</div>';
    container.innerHTML = html;
  }

  function renderNotFound(path, container) {
    document.title = 'Document Not Found — PortProbe Documentation';
    container.innerHTML = `
      <div style="padding: 3rem 0; text-align: center;">
        <h1 style="border: none; margin-bottom: 1rem;">Document Not Found</h1>
        <p style="color: var(--color-text-secondary); margin-bottom: 2rem;">
          The requested documentation page <code>${path}</code> could not be found.
        </p>
        <a href="#/README.md" class="btn-primary" style="display: inline-flex;">
          Return to Documentation Portal Home
        </a>
      </div>
    `;
  }

  // Expose global app state
  window.PortProbeApp = {
    getCurrentPath: () => currentPath,
    scrollToSection: scrollToSection
  };

  document.addEventListener('DOMContentLoaded', init);
})();
