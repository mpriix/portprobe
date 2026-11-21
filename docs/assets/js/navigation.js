/**
 * PortProbe Navigation Manager
 * Renders hierarchical sidebar categories, active link indicators,
 * filter search in sidebar, breadcrumbs, and previous/next navigation buttons.
 */

window.PortProbeNav = (function () {
  let navData = [];
  let flatItems = [];

  function init(navigation) {
    navData = navigation || (window.PORTPROBE_NAVIGATION || []);
    flatItems = [];
    
    // Flatten items for linear prev/next traversal
    navData.forEach(cat => {
      cat.items.forEach(item => {
        flatItems.push({
          ...item,
          categoryTitle: cat.title,
          categoryId: cat.id
        });
      });
    });

    renderSidebar();
    setupFilter();
  }

  function renderSidebar() {
    const container = document.getElementById('sidebar-nav-container');
    if (!container) return;

    let html = '';
    navData.forEach(cat => {
      html += `
        <div class="nav-section" data-cat-id="${cat.id}">
          <div class="nav-section-header" onclick="PortProbeNav.toggleSection('${cat.id}')">
            <span class="nav-section-title">
              ${getCategoryIcon(cat.icon)}
              ${cat.title}
            </span>
            <svg class="nav-chevron" xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>
          </div>
          <ul class="nav-items-list">
            ${cat.items.map(item => `
              <li class="nav-item">
                <a href="#/${item.path}" class="nav-link" data-path="${item.path}" title="${item.title}">
                  ${item.title}
                </a>
              </li>
            `).join('')}
          </ul>
        </div>
      `;
    });

    container.innerHTML = html;

    // Close drawer on link click in mobile view
    container.querySelectorAll('.nav-link').forEach(link => {
      link.addEventListener('click', () => {
        const sidebar = document.getElementById('app-sidebar');
        const backdrop = document.getElementById('sidebar-backdrop');
        if (sidebar) sidebar.classList.remove('drawer-open');
        if (backdrop) backdrop.classList.remove('open');
        document.body.classList.remove('drawer-open');
      });
    });
  }

  function getCategoryIcon(iconName) {
    const icons = {
      'book-open': '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/></svg>',
      'layers': '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="12 2 2 7 12 12 22 7 12 2"/><polyline points="2 17 12 22 22 17"/><polyline points="2 12 12 17 22 12"/></svg>',
      'play-circle': '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polygon points="10 8 16 12 10 16 10 8"/></svg>',
      'terminal': '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"/><line x1="12" x2="20" y1="19" y2="19"/></svg>',
      'file-code': '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="m10 13-2 2 2 2"/><path d="m14 17 2-2-2-2"/></svg>',
      'server': '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="20" height="8" x="2" y="2" rx="2" ry="2"/><rect width="20" height="8" x="2" y="14" rx="2" ry="2"/><line x1="6" x2="6.01" y1="6" y2="6"/><line x1="6" x2="6.01" y1="18" y2="18"/></svg>',
      'shield-check': '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="m9 12 2 2 4-4"/></svg>',
      'lock': '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>',
      'help-circle': '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><path d="M12 17h.01"/></svg>',
      'code-2': '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m18 16 4-4-4-4"/><path d="m6 8-4 4 4 4"/><path d="m14.5 4-5 16"/></svg>',
      'git-branch': '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="6" x2="6" y1="3" y2="15"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 0 1-9 9"/></svg>'
    };
    return icons[iconName] || icons['book-open'];
  }

  function toggleSection(catId) {
    const sec = document.querySelector(`.nav-section[data-cat-id="${catId}"]`);
    if (sec) {
      sec.classList.toggle('collapsed');
    }
  }

  function setActive(currentPath) {
    // Normalize path
    const cleanPath = currentPath.replace(/^#\/?/, '').replace(/^\//, '');
    
    document.querySelectorAll('.nav-link').forEach(link => {
      const linkPath = link.getAttribute('data-path');
      if (linkPath === cleanPath) {
        link.classList.add('active');
        // Ensure parent section is expanded
        const parentSec = link.closest('.nav-section');
        if (parentSec) {
          parentSec.classList.remove('collapsed');
        }
      } else {
        link.classList.remove('active');
      }
    });

    renderBreadcrumbs(cleanPath);
    renderFooterNav(cleanPath);
  }

  function renderBreadcrumbs(currentPath) {
    const el = document.getElementById('doc-breadcrumbs');
    if (!el) return;

    if (!currentPath || currentPath === 'README.md' || currentPath === '') {
      el.innerHTML = `<span>Docs</span> <span class="breadcrumb-separator">/</span> <span class="breadcrumb-active">Portal Home</span>`;
      return;
    }

    const item = flatItems.find(i => i.path === currentPath);
    if (item) {
      el.innerHTML = `
        <a href="#/README.md">Docs</a>
        <span class="breadcrumb-separator">/</span>
        <span>${item.categoryTitle}</span>
        <span class="breadcrumb-separator">/</span>
        <span class="breadcrumb-active">${item.title}</span>
      `;
    } else {
      el.innerHTML = `<a href="#/README.md">Docs</a> <span class="breadcrumb-separator">/</span> <span class="breadcrumb-active">${currentPath}</span>`;
    }
  }

  function renderFooterNav(currentPath) {
    const el = document.getElementById('doc-footer-nav');
    if (!el) return;

    const idx = flatItems.findIndex(i => i.path === currentPath);
    if (idx === -1) {
      el.innerHTML = '';
      return;
    }

    const prev = idx > 0 ? flatItems[idx - 1] : null;
    const next = idx < flatItems.length - 1 ? flatItems[idx + 1] : null;

    let html = '';
    if (prev) {
      html += `
        <a href="#/${prev.path}" class="footer-nav-card prev">
          <span class="footer-nav-direction">← Previous</span>
          <span class="footer-nav-title">${prev.title}</span>
        </a>
      `;
    } else {
      html += '<div></div>';
    }

    if (next) {
      html += `
        <a href="#/${next.path}" class="footer-nav-card next">
          <span class="footer-nav-direction">Next →</span>
          <span class="footer-nav-title">${next.title}</span>
        </a>
      `;
    }

    el.innerHTML = html;
  }

  function setupFilter() {
    const input = document.getElementById('nav-filter-input');
    if (!input) return;

    input.addEventListener('input', (e) => {
      const q = e.target.value.toLowerCase().trim();
      document.querySelectorAll('.nav-section').forEach(sec => {
        let hasVisible = false;
        sec.querySelectorAll('.nav-item').forEach(item => {
          const text = item.textContent.toLowerCase();
          if (!q || text.includes(q)) {
            item.style.display = 'block';
            hasVisible = true;
          } else {
            item.style.display = 'none';
          }
        });
        if (hasVisible) {
          sec.style.display = 'block';
          sec.classList.remove('collapsed');
        } else {
          sec.style.display = 'none';
        }
      });
    });
  }

  return {
    init,
    setActive,
    toggleSection
  };
})();
