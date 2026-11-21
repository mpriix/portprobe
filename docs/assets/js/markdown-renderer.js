/**
 * PortProbe Markdown Rendering Engine
 * Integrates marked.js, highlight.js, mermaid.js, GitHub Alert callouts,
 * language headers, copy code buttons, relative link resolution, and anchor navigation.
 */

window.PortProbeRenderer = (function () {
  function init() {
    if (!window.marked) {
      console.error('Marked.js not loaded!');
      return;
    }

    // Configure marked options
    marked.setOptions({
      gfm: true,
      breaks: false,
      highlight: function (code, lang) {
        if (window.hljs && lang && hljs.getLanguage(lang)) {
          try {
            return hljs.highlight(code, { language: lang }).value;
          } catch (e) {
            console.error('Highlight error:', e);
          }
        }
        return code;
      }
    });
  }

  /**
   * Rewrites internal markdown links to work seamlessly with hash routing:
   * e.g., "../product/architecture.md" -> "#/product/architecture.md"
   * e.g., "#expected-terminal-output" -> "#/currentDocPath#expected-terminal-output"
   * e.g., "installation.md#verification" -> "#/getting-started/installation.md#verification"
   */
  function rewriteInternalLinks(html, currentDocPath) {
    return html.replace(/<a\s+(?:[^>]*?\s+)?href="([^"]*)"([^>]*)>/gi, (match, href, rest) => {
      if (href.startsWith('http://') || href.startsWith('https://') || href.startsWith('mailto:')) {
        return `<a href="${href}" target="_blank" rel="noopener noreferrer"${rest}>`;
      }
      
      // Bare in-page anchor
      if (href.startsWith('#')) {
        const anchor = href.slice(1);
        const fullHref = currentDocPath ? `#/${currentDocPath}#${anchor}` : `#${anchor}`;
        return `<a href="${fullHref}" data-anchor="${anchor}"${rest}>`;
      }

      // External repo source code links (e.g. ../src/main.rs or ../../src/cli.rs)
      if (href.includes('/src/') || href.startsWith('../src/') || href.startsWith('../../src/')) {
        const cleanHref = href.replace(/^(\.\.\/)+/, '');
        return `<a href="https://github.com/portprobe/portprobe/blob/master/${cleanHref}" target="_blank" rel="noopener noreferrer"${rest}>`;
      }
      
      // Resolve relative path and section anchor
      let targetPath = href;
      let anchor = '';
      if (targetPath.includes('#')) {
        const hashIdx = targetPath.indexOf('#');
        anchor = targetPath.slice(hashIdx + 1);
        targetPath = targetPath.slice(0, hashIdx);
      }

      if (targetPath.endsWith('.md')) {
        if (targetPath.startsWith('../') || targetPath.startsWith('./')) {
          const currentDir = currentDocPath.includes('/') ? currentDocPath.substring(0, currentDocPath.lastIndexOf('/')) : '';
          const parts = (currentDir + '/' + targetPath).split('/');
          const resolved = [];
          for (const p of parts) {
            if (!p || p === '.') continue;
            if (p === '..') {
              resolved.pop();
            } else {
              resolved.push(p);
            }
          }
          targetPath = resolved.join('/');
        }
        const fullHref = anchor ? `#/${targetPath}#${anchor}` : `#/${targetPath}`;
        return `<a href="${fullHref}"${rest}>`;
      }
      return match;
    });
  }

  /**
   * Post-processes HTML:
   * 1. Converts GitHub Alert blockquotes (> [!NOTE]) into styled alert callouts.
   * 2. Wraps code blocks with language labels and one-click copy buttons.
   * 3. Wraps tables in responsive horizontal-scroll containers.
   * 4. Adds clickable heading anchors that do not disrupt hash routing.
   * 5. Handles in-content anchor link clicks smoothly without triggering full route transitions.
   */
  function postprocessHtml(container, currentDocPath) {
    // 1. Process GitHub Alert Callouts (> [!NOTE], [!TIP], [!IMPORTANT], [!WARNING], [!CAUTION])
    const alertTitles = {
      note: 'Note',
      tip: 'Tip',
      important: 'Important',
      warning: 'Warning',
      caution: 'Caution'
    };

    const alertIcons = {
      note: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/></svg>',
      tip: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 14c.2-1 .7-1.7 1.5-2.5 1-.9 1.5-2.2 1.5-3.5A6 6 0 0 0 6 8c0 1 .2 2.2 1.5 3.5.7.7 1.3 1.5 1.5 2.5"/><path d="M9 18h6"/><path d="M10 22h4"/></svg>',
      important: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" x2="12" y1="8" y2="12"/><line x1="12" x2="12.01" y1="16" y2="16"/></svg>',
      warning: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><line x1="12" x2="12" y1="9" y2="13"/><line x1="12" x2="12.01" y1="17" y2="17"/></svg>',
      caution: '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="7.86 2 16.14 2 22 7.86 22 16.14 16.14 22 7.86 22 2 16.14 2 7.86 7.86 2"/><line x1="12" x2="12" y1="8" y2="12"/><line x1="12" x2="12.01" y1="16" y2="16"/></svg>'
    };

    container.querySelectorAll('blockquote').forEach((bq) => {
      const html = bq.innerHTML.trim();
      const match = html.match(/^\s*<p>\s*\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\]\s*(?:<br\s*\/?>)?\s*([\s\S]*?)<\/p>/i);
      
      if (match) {
        const alertType = match[1].toLowerCase();
        
        // Strip the [!NOTE] tag and keep the body
        let remainingHtml = html.replace(/^\s*<p>\s*\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\]\s*(?:<br\s*\/?>)?\s*/i, '<p>');
        remainingHtml = remainingHtml.replace(/^\s*<p>\s*<\/p>\s*/i, '');

        const alertDiv = document.createElement('div');
        alertDiv.className = `alert-callout ${alertType}`;
        alertDiv.innerHTML = `
          <div class="alert-header">
            ${alertIcons[alertType] || alertIcons.note}
            <span>${alertTitles[alertType] || 'Note'}</span>
          </div>
          <div class="alert-body">
            ${remainingHtml}
          </div>
        `;

        bq.replaceWith(alertDiv);
      }
    });

    // 2. Process Code Blocks (Syntax highlighting, language badge, copy button)
    container.querySelectorAll('pre code').forEach((codeBlock) => {
      const pre = codeBlock.parentElement;
      if (pre.classList.contains('mermaid') || pre.parentElement.classList.contains('code-block-container')) {
        return;
      }

      // Check if this is a mermaid diagram block
      const codeText = codeBlock.textContent.trim();
      const isMermaid = codeBlock.classList.contains('language-mermaid') || 
                        codeText.startsWith('flowchart') || 
                        codeText.startsWith('sequenceDiagram') ||
                        codeText.startsWith('classDiagram') ||
                        codeText.startsWith('stateDiagram');

      if (isMermaid) {
        const mermaidDiv = document.createElement('div');
        mermaidDiv.className = 'mermaid-container';
        mermaidDiv.setAttribute('data-mermaid-source', codeText);

        mermaidDiv.innerHTML = `
          <div class="diagram-toolbar">
            <span class="diagram-badge">
              <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/><path d="M3 9h18"/><path d="M9 21V9"/></svg>
              <span>Architecture Diagram</span>
            </span>
            <div class="diagram-actions">
              <button class="diagram-btn zoom-out-btn" title="Zoom Out" aria-label="Zoom out">
                <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="5" x2="19" y1="12" y2="12"/></svg>
              </button>
              <button class="diagram-btn zoom-reset-btn" title="Reset View" aria-label="Reset zoom">
                <span class="zoom-val">100%</span>
              </button>
              <button class="diagram-btn zoom-in-btn" title="Zoom In" aria-label="Zoom in">
                <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" x2="12" y1="5" y2="19"/><line x1="5" x2="19" y1="12" y2="12"/></svg>
              </button>
              <button class="diagram-btn toggle-scroll-btn" title="Toggle Full Width / Scroll" aria-label="Expand diagram">
                <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 3 21 3 21 9"/><polyline points="9 21 3 21 3 15"/><line x1="21" x2="14" y1="3" y2="10"/><line x1="3" x2="10" y1="21" y2="14"/></svg>
                <span>Expand</span>
              </button>
            </div>
          </div>
          <div class="diagram-viewport">
            <pre class="mermaid">${codeText}</pre>
          </div>
        `;

        pre.replaceWith(mermaidDiv);
        return;
      }

      // Extract language label
      let lang = 'TEXT';
      codeBlock.classList.forEach((cls) => {
        if (cls.startsWith('language-')) {
          lang = cls.replace('language-', '').toUpperCase();
        }
      });

      // Wrap code block with enterprise header
      const wrapper = document.createElement('div');
      wrapper.className = 'code-block-container';

      const header = document.createElement('div');
      header.className = 'code-block-header';
      header.innerHTML = `
        <span class="code-lang-label">${lang}</span>
        <button class="copy-btn" title="Copy code snippet">
          <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>
          <span class="copy-text">Copy</span>
        </button>
      `;

      const copyBtn = header.querySelector('.copy-btn');
      copyBtn.addEventListener('click', () => {
        navigator.clipboard.writeText(codeBlock.textContent).then(() => {
          copyBtn.innerHTML = `
            <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#10B981" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
            <span style="color: #10B981; font-weight: 600;">Copied!</span>
          `;
          setTimeout(() => {
            copyBtn.innerHTML = `
              <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>
              <span class="copy-text">Copy</span>
            `;
          }, 2000);
        });
      });

      pre.replaceWith(wrapper);
      wrapper.appendChild(header);
      wrapper.appendChild(pre);
    });

    // 3. Process Tables
    container.querySelectorAll('table').forEach((table) => {
      if (!table.parentElement.classList.contains('table-wrapper')) {
        const wrapper = document.createElement('div');
        wrapper.className = 'table-wrapper';
        table.replaceWith(wrapper);
        wrapper.appendChild(table);
      }
    });

    // 4. Process Headings for Anchors
    container.querySelectorAll('h2, h3, h4').forEach((h) => {
      if (!h.id) {
        h.id = h.textContent.toLowerCase().replace(/[^\w\s-]/g, '').replace(/[-\s]+/g, '-').replace(/^-+|-+$/g, '');
      }
      if (!h.querySelector('.heading-anchor')) {
        const anchor = document.createElement('a');
        anchor.className = 'heading-anchor';
        anchor.href = `#${h.id}`;
        anchor.title = 'Copy section link';
        anchor.innerHTML = '#';
        anchor.addEventListener('click', (e) => {
          e.preventDefault();
          const docPath = window.PortProbeApp ? window.PortProbeApp.getCurrentPath() : '';
          const fullUrl = window.location.origin + window.location.pathname + `#/${docPath}#${h.id}`;
          navigator.clipboard.writeText(fullUrl);
          const yOffset = -70;
          const y = h.getBoundingClientRect().top + window.pageYOffset + yOffset;
          window.scrollTo({ top: y, behavior: 'smooth' });
          if (docPath) {
            history.replaceState(null, '', `#/${docPath}#${h.id}`);
          }
        });
        h.appendChild(anchor);
      }
    });

    // 5. In-content internal anchor links smooth scroll
    container.querySelectorAll('a[data-anchor]').forEach((a) => {
      a.addEventListener('click', (e) => {
        const anchorId = a.getAttribute('data-anchor');
        const target = document.getElementById(anchorId);
        if (target) {
          e.preventDefault();
          const yOffset = -70;
          const y = target.getBoundingClientRect().top + window.pageYOffset + yOffset;
          window.scrollTo({ top: y, behavior: 'smooth' });
          const docPath = window.PortProbeApp ? window.PortProbeApp.getCurrentPath() : '';
          if (docPath) {
            history.replaceState(null, '', `#/${docPath}#${anchorId}`);
          }
        }
      });
    });
  }

  async function renderMermaid() {
    if (!window.mermaid) return;
    const containers = document.querySelectorAll('.mermaid-container');
    if (!containers.length) return;

    // Reset each container to clean raw source so theme change / re-render works reliably
    containers.forEach((container, idx) => {
      const src = container.getAttribute('data-mermaid-source');
      if (src) {
        let viewport = container.querySelector('.diagram-viewport');
        if (!viewport) {
          viewport = document.createElement('div');
          viewport.className = 'diagram-viewport';
          container.appendChild(viewport);
        }
        viewport.innerHTML = `<pre class="mermaid" id="mermaid-diag-${idx}">${src.trim()}</pre>`;
      }
    });

    try {
      await mermaid.run({
        nodes: document.querySelectorAll('.mermaid')
      });
    } catch (e) {
      console.warn('Mermaid rendering notice:', e);
    }

    // Attach interactive zoom and view controls to each diagram container
    containers.forEach((container) => {
      const viewport = container.querySelector('.diagram-viewport');
      const svg = viewport ? viewport.querySelector('svg') : null;
      if (!viewport || !svg) return;

      let zoomLevel = 1.0;
      const zoomValBadge = container.querySelector('.zoom-val');

      const applyZoom = (lvl) => {
        zoomLevel = Math.max(0.5, Math.min(2.5, lvl));
        svg.style.transform = `scale(${zoomLevel})`;
        svg.style.transformOrigin = 'top center';
        if (zoomValBadge) {
          zoomValBadge.textContent = `${Math.round(zoomLevel * 100)}%`;
        }
      };

      const inBtn = container.querySelector('.zoom-in-btn');
      if (inBtn && !inBtn._bound) {
        inBtn._bound = true;
        inBtn.addEventListener('click', () => applyZoom(zoomLevel + 0.15));
      }

      const outBtn = container.querySelector('.zoom-out-btn');
      if (outBtn && !outBtn._bound) {
        outBtn._bound = true;
        outBtn.addEventListener('click', () => applyZoom(zoomLevel - 0.15));
      }

      const resetBtn = container.querySelector('.zoom-reset-btn');
      if (resetBtn && !resetBtn._bound) {
        resetBtn._bound = true;
        resetBtn.addEventListener('click', () => applyZoom(1.0));
      }

      const toggleBtn = container.querySelector('.toggle-scroll-btn');
      if (toggleBtn && !toggleBtn._bound) {
        toggleBtn._bound = true;
        toggleBtn.addEventListener('click', () => {
          viewport.classList.toggle('scrollable');
          const isScrollable = viewport.classList.contains('scrollable');
          toggleBtn.innerHTML = isScrollable
            ? '<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 14h6v6"/><path d="M20 10h-6V4"/><path d="m14 10 7-7"/><path d="m3 21 7-7"/></svg><span>Fit</span>'
            : '<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 3 21 3 21 9"/><polyline points="9 21 3 21 3 15"/><line x1="21" x2="14" y1="3" y2="10"/><line x1="3" x2="10" y1="21" y2="14"/></svg><span>Expand</span>';
        });
      }
    });
  }

  function render(markdown, currentPath) {
    let rawHtml = marked.parse(markdown);
    rawHtml = rewriteInternalLinks(rawHtml, currentPath);
    return rawHtml;
  }

  return {
    init,
    render,
    postprocessHtml,
    renderMermaid
  };
})();
