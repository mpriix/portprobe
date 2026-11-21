/**
 * PortProbe Table of Contents (TOC) Manager
 * Generates dynamic page-level outline from H2 and H3 headings,
 * highlights current scroll position with IntersectionObserver,
 * attaches smooth scrolling handlers that preserve routing,
 * and handles smooth Back to Top scrolling.
 */

window.PortProbeToc = (function () {
  let observer = null;
  let activeDocPath = '';

  function generate(headings, currentDocPath) {
    activeDocPath = currentDocPath || '';
    const tocList = document.getElementById('toc-list');
    const tocContainer = document.getElementById('app-toc-container');
    if (!tocList || !tocContainer) return;

    if (!headings || headings.length === 0) {
      tocContainer.style.display = 'none';
      return;
    }

    tocContainer.style.display = 'block';

    let html = '';
    headings.forEach((h) => {
      const levelClass = h.level === 3 ? 'level-3' : 'level-2';
      const targetHref = activeDocPath ? `#/${activeDocPath}#${h.id}` : `#${h.id}`;
      html += `
        <li class="toc-item ${levelClass}">
          <a href="${targetHref}" class="toc-link" data-id="${h.id}" title="${escapeHtml(h.text)}">
            ${escapeHtml(h.text)}
          </a>
        </li>
      `;
    });

    tocList.innerHTML = html;

    // Attach click listeners to TOC links for smooth scrolling without breaking SPA hash route
    tocList.querySelectorAll('.toc-link').forEach((link) => {
      link.addEventListener('click', (e) => {
        e.preventDefault();
        const sectionId = link.getAttribute('data-id');
        const targetEl = document.getElementById(sectionId);
        if (targetEl) {
          const yOffset = -70; // Header offset
          const y = targetEl.getBoundingClientRect().top + window.pageYOffset + yOffset;
          window.scrollTo({ top: y, behavior: 'smooth' });

          // Update URL hash cleanly without causing route reload/rerender
          if (activeDocPath) {
            history.replaceState(null, '', `#/${activeDocPath}#${sectionId}`);
          }

          // Update active state in TOC
          tocList.querySelectorAll('.toc-link').forEach((l) => l.classList.remove('active'));
          link.classList.add('active');
        }
      });
    });

    setupScrollSpy();
    setupBackToTop();
  }

  function escapeHtml(str) {
    if (!str) return '';
    return str
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  function setupScrollSpy() {
    if (observer) {
      observer.disconnect();
    }

    const headingElements = document.querySelectorAll('.markdown-body h2, .markdown-body h3');
    if (headingElements.length === 0) return;

    observer = new IntersectionObserver((entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          const id = entry.target.id;
          document.querySelectorAll('.toc-link').forEach((link) => {
            if (link.getAttribute('data-id') === id) {
              link.classList.add('active');
            } else {
              link.classList.remove('active');
            }
          });
        }
      });
    }, {
      rootMargin: '-80px 0px -70% 0px',
      threshold: 0
    });

    headingElements.forEach((el) => observer.observe(el));
  }

  function setupBackToTop() {
    const btn = document.getElementById('back-to-top-btn');
    if (!btn) return;

    btn.onclick = () => {
      window.scrollTo({ top: 0, behavior: 'smooth' });
    };
  }

  return {
    generate
  };
})();
