/**
 * PortProbe Client-Side Documentation Search
 * Provides full-text searching across titles, headings, and document content
 * with keyboard shortcuts (Ctrl+K, /), snippet extraction, and term highlighting.
 */

window.PortProbeSearch = (function () {
  let searchIndex = [];
  let modalBackdrop = null;
  let searchInput = null;
  let resultsContainer = null;
  let selectedIndex = -1;
  let currentResults = [];

  function init(indexData) {
    searchIndex = indexData || (window.PORTPROBE_SEARCH_INDEX || []);
    setupModal();
    setupKeyboardShortcuts();
  }

  function setupModal() {
    modalBackdrop = document.getElementById('search-modal-backdrop');
    searchInput = document.getElementById('search-modal-input');
    resultsContainer = document.getElementById('search-modal-results');

    if (!modalBackdrop || !searchInput || !resultsContainer) return;

    // Close on backdrop click
    modalBackdrop.addEventListener('click', (e) => {
      if (e.target === modalBackdrop) {
        closeModal();
      }
    });

    const closeBtn = document.getElementById('search-modal-close-btn');
    if (closeBtn) {
      closeBtn.addEventListener('click', closeModal);
    }

    searchInput.addEventListener('input', (e) => {
      performSearch(e.target.value.trim());
    });

    searchInput.addEventListener('keydown', handleKeyNavigation);

    // Global trigger button in header
    const triggerBtn = document.getElementById('header-search-btn');
    if (triggerBtn) {
      triggerBtn.addEventListener('click', openModal);
    }

    // Mobile search button
    const mobileTriggerBtn = document.getElementById('mobile-search-btn');
    if (mobileTriggerBtn) {
      mobileTriggerBtn.addEventListener('click', openModal);
    }
  }

  function setupKeyboardShortcuts() {
    document.addEventListener('keydown', (e) => {
      // Ctrl+K or Cmd+K
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        openModal();
      }
      // Slash (/) key when not in an input
      else if (e.key === '/' && document.activeElement.tagName !== 'INPUT' && document.activeElement.tagName !== 'TEXTAREA') {
        e.preventDefault();
        openModal();
      }
      // Escape to close
      else if (e.key === 'Escape' && modalBackdrop.classList.contains('open')) {
        closeModal();
      }
    });
  }

  function openModal() {
    if (!modalBackdrop) return;
    modalBackdrop.classList.add('open');
    searchInput.value = '';
    selectedIndex = -1;
    currentResults = [];
    resultsContainer.innerHTML = `
      <div class="search-empty-state">
        <p>Type keywords to search across PortProbe documentation...</p>
        <p style="margin-top: 0.5rem; font-size: 0.75rem; color: var(--color-text-dim);">
          Tip: Try searching for <code>rate limit</code>, <code>systemd</code>, <code>mTLS</code>, <code>exit codes</code>, or <code>port ranges</code>.
        </p>
      </div>
    `;
    setTimeout(() => searchInput.focus(), 50);
  }

  function closeModal() {
    if (!modalBackdrop) return;
    modalBackdrop.classList.remove('open');
  }

  function escapeRegExp(string) {
    return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  }

  function highlightMatches(text, query) {
    if (!query) return text;
    const regex = new RegExp(`(${escapeRegExp(query)})`, 'gi');
    return text.replace(regex, '<mark class="search-highlight">$1</mark>');
  }

  function extractSnippet(content, query) {
    const lowerContent = content.toLowerCase();
    const lowerQuery = query.toLowerCase();
    const pos = lowerContent.indexOf(lowerQuery);
    
    if (pos === -1) {
      return content.substring(0, 140) + '...';
    }

    const start = Math.max(0, pos - 50);
    const end = Math.min(content.length, pos + query.length + 80);
    let snippet = content.substring(start, end).trim();
    if (start > 0) snippet = '...' + snippet;
    if (end < content.length) snippet = snippet + '...';
    return snippet;
  }

  function performSearch(query) {
    selectedIndex = -1;
    if (!query || query.length < 2) {
      currentResults = [];
      resultsContainer.innerHTML = `
        <div class="search-empty-state">
          <p>Please enter at least 2 characters to search.</p>
        </div>
      `;
      return;
    }

    const lowerQuery = query.toLowerCase();
    const terms = lowerQuery.split(/\s+/).filter(t => t.length > 0);

    const matches = [];

    searchIndex.forEach((doc) => {
      let score = 0;
      const lowerTitle = doc.title.toLowerCase();
      const lowerContent = doc.content.toLowerCase();
      const headingsJoined = doc.headings.join(' ').toLowerCase();

      // Title match (highest weight)
      if (lowerTitle.includes(lowerQuery)) score += 50;
      // Exact term matches in title
      terms.forEach(t => {
        if (lowerTitle.includes(t)) score += 20;
      });

      // Headings match (medium weight)
      terms.forEach(t => {
        if (headingsJoined.includes(t)) score += 10;
      });

      // Content match
      terms.forEach(t => {
        if (lowerContent.includes(t)) score += 2;
      });

      if (score > 0) {
        matches.push({
          doc,
          score,
          snippet: extractSnippet(doc.content, query)
        });
      }
    });

    matches.sort((a, b) => b.score - a.score);
    currentResults = matches.slice(0, 15);

    renderResults(query);
  }

  function renderResults(query) {
    if (currentResults.length === 0) {
      resultsContainer.innerHTML = `
        <div class="search-empty-state">
          <p>No matching documentation found for "<strong>${escapeHtml(query)}</strong>"</p>
          <p style="margin-top: 0.5rem; font-size: 0.75rem; color: var(--color-text-dim);">
            Verify spelling or try broader terms like <code>cli</code>, <code>tls</code>, or <code>firewall</code>.
          </p>
        </div>
      `;
      return;
    }

    let html = '';
    currentResults.forEach((res, idx) => {
      const doc = res.doc;
      html += `
        <a href="#/${doc.path}" class="search-result-item ${idx === selectedIndex ? 'selected' : ''}" data-idx="${idx}" onclick="PortProbeSearch.close()">
          <div class="result-category">${doc.category}</div>
          <div class="result-title">${highlightMatches(doc.title, query)}</div>
          <div class="result-snippet">${highlightMatches(res.snippet, query)}</div>
        </a>
      `;
    });

    resultsContainer.innerHTML = html;
  }

  function handleKeyNavigation(e) {
    if (currentResults.length === 0) return;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = (selectedIndex + 1) % currentResults.length;
      updateSelectedVisual();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = (selectedIndex - 1 + currentResults.length) % currentResults.length;
      updateSelectedVisual();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (selectedIndex >= 0 && selectedIndex < currentResults.length) {
        const item = currentResults[selectedIndex];
        window.location.hash = `#/${item.doc.path}`;
        closeModal();
      }
    }
  }

  function updateSelectedVisual() {
    const items = resultsContainer.querySelectorAll('.search-result-item');
    items.forEach((item, idx) => {
      if (idx === selectedIndex) {
        item.classList.add('selected');
        item.scrollIntoView({ block: 'nearest' });
      } else {
        item.classList.remove('selected');
      }
    });
  }

  function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  return {
    init,
    open: openModal,
    close: closeModal
  };
})();
