/**
 * PortProbe Theme Manager
 * Manages light, dark, and system themes with localStorage persistence
 * and re-triggers Mermaid diagram rendering upon theme changes.
 */

(function () {
  const THEME_STORAGE_KEY = 'portprobe_docs_theme';

  function getPreferredTheme() {
    const saved = localStorage.getItem(THEME_STORAGE_KEY);
    if (saved === 'light' || saved === 'dark') {
      return saved;
    }
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }

  function applyTheme(theme) {
    document.documentElement.setAttribute('data-theme', theme);
    const themeIcon = document.getElementById('theme-toggle-icon');
    if (themeIcon) {
      themeIcon.innerHTML = theme === 'dark' 
        ? '<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/></svg>'
        : '<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/></svg>';
    }

    // Re-render Mermaid diagrams if mermaid is initialized
    if (window.mermaid) {
      try {
        const isDark = theme === 'dark';
        window.mermaid.initialize({
          startOnLoad: false,
          securityLevel: 'loose',
          fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif',
          theme: 'base',
          themeVariables: isDark ? {
            darkMode: true,
            background: '#0B0F17',
            mainBkg: '#1E293B',
            primaryColor: '#1E293B',
            primaryTextColor: '#F8FAFC',
            primaryBorderColor: '#38BDF8',
            lineColor: '#94A3B8',
            secondaryColor: '#0F172A',
            secondaryTextColor: '#F8FAFC',
            secondaryBorderColor: '#0284C7',
            tertiaryColor: '#1E293B',
            tertiaryTextColor: '#F8FAFC',
            tertiaryBorderColor: '#334155',
            nodeBorder: '#38BDF8',
            clusterBkg: '#0F172A',
            clusterBorder: '#334155',
            titleColor: '#38BDF8',
            edgeLabelBackground: '#0F172A',
            nodeTextColor: '#F8FAFC',
            actorTextColor: '#F8FAFC',
            actorBkg: '#1E293B',
            actorBorder: '#38BDF8',
            signalColor: '#94A3B8',
            signalTextColor: '#F8FAFC',
            labelTextColor: '#CBD5E1',
            altBackground: '#0F172A',
            fontSize: '13px'
          } : {
            darkMode: false,
            background: '#FFFFFF',
            mainBkg: '#F0F9FF',
            primaryColor: '#F0F9FF',
            primaryTextColor: '#0F172A',
            primaryBorderColor: '#0284C7',
            lineColor: '#475569',
            secondaryColor: '#F8FAFC',
            secondaryTextColor: '#0F172A',
            secondaryBorderColor: '#CBD5E1',
            tertiaryColor: '#FFFFFF',
            tertiaryTextColor: '#0F172A',
            tertiaryBorderColor: '#E2E8F0',
            nodeBorder: '#0284C7',
            clusterBkg: '#F8FAFC',
            clusterBorder: '#CBD5E1',
            titleColor: '#0369A1',
            edgeLabelBackground: '#FFFFFF',
            nodeTextColor: '#0F172A',
            actorTextColor: '#0F172A',
            actorBkg: '#F0F9FF',
            actorBorder: '#0284C7',
            signalColor: '#475569',
            signalTextColor: '#0F172A',
            labelTextColor: '#1E293B',
            altBackground: '#F8FAFC',
            fontSize: '13px'
          }
        });
        if (window.PortProbeRenderer && window.PortProbeRenderer.renderMermaid) {
          window.PortProbeRenderer.renderMermaid();
        }
      } catch (e) {
        console.warn('Mermaid re-init deferred:', e);
      }
    }
  }

  function toggleTheme() {
    const current = document.documentElement.getAttribute('data-theme') || 'dark';
    const next = current === 'dark' ? 'light' : 'dark';
    localStorage.setItem(THEME_STORAGE_KEY, next);
    applyTheme(next);
  }

  // Initial theme application before DOM fully loads to prevent flash
  const initialTheme = getPreferredTheme();
  document.documentElement.setAttribute('data-theme', initialTheme);

  document.addEventListener('DOMContentLoaded', () => {
    applyTheme(initialTheme);
    const toggleBtn = document.getElementById('theme-toggle-btn');
    if (toggleBtn) {
      toggleBtn.addEventListener('click', toggleTheme);
    }

    // Listen for OS system theme changes
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
      if (!localStorage.getItem(THEME_STORAGE_KEY)) {
        applyTheme(e.matches ? 'dark' : 'light');
      }
    });
  });

  window.PortProbeTheme = {
    get: () => document.documentElement.getAttribute('data-theme'),
    set: (t) => {
      localStorage.setItem(THEME_STORAGE_KEY, t);
      applyTheme(t);
    },
    toggle: toggleTheme
  };
})();
