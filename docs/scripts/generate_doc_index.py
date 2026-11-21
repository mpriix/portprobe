#!/usr/bin/env python3
"""
PortProbe Documentation Index & Bundle Generator.
Scans the docs/ directory, parses metadata, headings, and content from all Markdown files,
builds a hierarchical navigation tree, pre-computes a search index, copies markdown files,
and bundles everything into assets/js/docs-bundle.js and config/navigation.json.
"""

import os
import re
import json
import shutil
from pathlib import Path

# Category configuration and display ordering
CATEGORY_CONFIG = [
    {
        "id": "overview",
        "title": "Overview",
        "icon": "book-open",
        "order": 1,
        "files": [
            {"path": "README.md", "title": "Portal Home"},
            {"path": "DOCUMENTATION_AUDIT_REPORT.md", "title": "Documentation Audit Report"}
        ]
    },
    {
        "id": "product",
        "title": "Product Architecture",
        "icon": "layers",
        "order": 2,
        "files": [
            {"path": "product/product-overview.md", "title": "Product Overview"},
            {"path": "product/architecture.md", "title": "System Architecture"},
            {"path": "product/technical-design.md", "title": "Technical Design"}
        ]
    },
    {
        "id": "getting-started",
        "title": "Getting Started",
        "icon": "play-circle",
        "order": 3,
        "files": [
            {"path": "getting-started/prerequisites.md", "title": "System Prerequisites"},
            {"path": "getting-started/installation.md", "title": "Installation Guide"},
            {"path": "getting-started/quick-start.md", "title": "Quick-Start Guide"}
        ]
    },
    {
        "id": "user-guide",
        "title": "User Guide",
        "icon": "terminal",
        "order": 4,
        "files": [
            {"path": "user-guide/overview.md", "title": "User Guide Overview"},
            {"path": "user-guide/daemon-management.md", "title": "Daemon Management"},
            {"path": "user-guide/client-usage.md", "title": "Client Probe Mode"},
            {"path": "user-guide/connectivity-testing.md", "title": "Connectivity Testing"},
            {"path": "user-guide/concurrency.md", "title": "Concurrency & Limits"},
            {"path": "user-guide/configuration.md", "title": "Configuration Architecture"},
            {"path": "user-guide/practical-examples.md", "title": "Practical Recipes"}
        ]
    },
    {
        "id": "reference",
        "title": "Reference Manuals",
        "icon": "file-code",
        "order": 5,
        "files": [
            {"path": "reference/cli-reference.md", "title": "CLI Reference"},
            {"path": "reference/configuration-reference.md", "title": "Configuration Reference"},
            {"path": "reference/error-reference.md", "title": "Error Reference"},
            {"path": "reference/exit-codes.md", "title": "Exit Codes"}
        ]
    },
    {
        "id": "deployment",
        "title": "Deployment & Operations",
        "icon": "server",
        "order": 6,
        "files": [
            {"path": "deployment/deployment-guide.md", "title": "Deployment Architecture"},
            {"path": "deployment/linux-service.md", "title": "Linux Service (systemd)"},
            {"path": "deployment/production-best-practices.md", "title": "Production Best Practices"},
            {"path": "deployment/performance-tuning.md", "title": "Performance Tuning"}
        ]
    },
    {
        "id": "examples",
        "title": "Real-World Scenarios",
        "icon": "shield-check",
        "order": 7,
        "files": [
            {"path": "examples/firewall-validation.md", "title": "Firewall Rule Validation"},
            {"path": "examples/load-balancer-validation.md", "title": "Load Balancer Probing"},
            {"path": "examples/infrastructure-validation.md", "title": "Infrastructure Validation"},
            {"path": "examples/ci-cd-integration.md", "title": "CI/CD Pipeline Integration"}
        ]
    },
    {
        "id": "security",
        "title": "Security & Compliance",
        "icon": "lock",
        "order": 8,
        "files": [
            {"path": "security/security-guide.md", "title": "Security Architecture"}
        ]
    },
    {
        "id": "troubleshooting",
        "title": "Troubleshooting & FAQ",
        "icon": "help-circle",
        "order": 9,
        "files": [
            {"path": "troubleshooting/troubleshooting-guide.md", "title": "Troubleshooting Guide"},
            {"path": "troubleshooting/faq.md", "title": "Frequently Asked Questions"}
        ]
    },
    {
        "id": "development",
        "title": "Developer Resources",
        "icon": "code-2",
        "order": 10,
        "files": [
            {"path": "development/developer-guide.md", "title": "Developer Architecture"},
            {"path": "development/build-and-test.md", "title": "Build & CI Quality Gates"},
            {"path": "development/contributing.md", "title": "Contributing Guidelines"}
        ]
    },
    {
        "id": "release-notes",
        "title": "Release Notes",
        "icon": "git-branch",
        "order": 11,
        "files": [
            {"path": "release-notes/changelog.md", "title": "Changelog & History"},
            {"path": "release-notes/known-limitations.md", "title": "Known Limitations"},
            {"path": "release-notes/roadmap.md", "title": "Product Roadmap"}
        ]
    }
]

def slugify(text):
    text = re.sub(r'[^\w\s-]', '', text.lower())
    return re.sub(r'[-\s]+', '-', text).strip('-')

def parse_markdown_metadata(content):
    """Extract H1 title, metadata badges, headings (H2, H3), and text content."""
    lines = content.split('\n')
    title = ""
    metadata = {}
    headings = []
    clean_lines = []
    
    in_code_block = False
    for line in lines:
        stripped = line.strip()
        if stripped.startswith('```'):
            in_code_block = not in_code_block
            continue
        if in_code_block:
            continue
            
        if not title and stripped.startswith('# '):
            title = stripped[2:].strip()
            # strip backticks or markdown formatting from title
            title = re.sub(r'[`*]', '', title)
            continue
            
        # Parse metadata lines e.g. **Document Reference:** `PORTPROBE-DOC-01`
        meta_match = re.match(r'\*\*([^:]+):\*\*\s*`?([^`\n*]+)`?', stripped)
        if meta_match:
            key = meta_match.group(1).strip()
            val = meta_match.group(2).strip()
            metadata[key] = val
            
        # Parse H2 and H3 headings for Table of Contents
        if stripped.startswith('## '):
            h_text = re.sub(r'[`*]', '', stripped[3:].strip())
            h_id = slugify(h_text)
            headings.append({"level": 2, "text": h_text, "id": h_id})
        elif stripped.startswith('### '):
            h_text = re.sub(r'[`*]', '', stripped[4:].strip())
            h_id = slugify(h_text)
            headings.append({"level": 3, "text": h_text, "id": h_id})
            
        clean_lines.append(line)
        
    plain_text = re.sub(r'[#*`_~\[\]\(\)\<\>]', ' ', '\n'.join(clean_lines))
    plain_text = re.sub(r'\s+', ' ', plain_text).strip()
    
    return {
        "title": title or "Untitled Document",
        "metadata": metadata,
        "headings": headings,
        "text": plain_text[:5000] # indexed snippet for search
    }

def build_documentation_bundle():
    docs_dir = Path(__file__).resolve().parent.parent
    docs_source = docs_dir
    assets_js = docs_dir / "assets" / "js"
    config_dir = docs_dir / "config"
    
    os.makedirs(assets_js, exist_ok=True)
    os.makedirs(config_dir, exist_ok=True)
    
    print(f"[*] Scanning source docs from: {docs_source}")
    
    docs_map = {}
    search_index = []
    navigation = []
    
    # Process each category
    for cat in CATEGORY_CONFIG:
        cat_entry = {
            "id": cat["id"],
            "title": cat["title"],
            "icon": cat["icon"],
            "order": cat["order"],
            "items": []
        }
        
        for file_info in cat["files"]:
            rel_path = file_info["path"]
            src_file = docs_source / rel_path
            
            if not src_file.exists():
                print(f"[!] Warning: File {rel_path} does not exist in {docs_source}! Skipping.")
                continue
            
            with open(src_file, "r", encoding="utf-8") as f:
                raw_content = f.read()
                
            parsed = parse_markdown_metadata(raw_content)
            doc_title = file_info.get("title") or parsed["title"]
            
            doc_id = rel_path.replace("/", "-").replace("\\", "-").replace(".md", "")
            
            docs_map[rel_path] = {
                "id": doc_id,
                "path": rel_path,
                "title": doc_title,
                "category": cat["title"],
                "categoryId": cat["id"],
                "metadata": parsed["metadata"],
                "headings": parsed["headings"],
                "markdown": raw_content
            }
            
            cat_entry["items"].append({
                "id": doc_id,
                "path": rel_path,
                "title": doc_title,
                "headings": parsed["headings"]
            })
            
            # Add to search index
            search_index.append({
                "id": doc_id,
                "path": rel_path,
                "title": doc_title,
                "category": cat["title"],
                "headings": [h["text"] for h in parsed["headings"]],
                "content": parsed["text"]
            })
            
        navigation.append(cat_entry)
        
    print(f"[+] Total documents indexed and bundled: {len(docs_map)}")
    
    # Write config/navigation.json
    nav_json_path = config_dir / "navigation.json"
    with open(nav_json_path, "w", encoding="utf-8") as f:
        json.dump(navigation, f, indent=2)
    print(f"[+] Saved navigation config to: {nav_json_path}")
    
    # Write search index json
    search_json_path = assets_js / "search-index.json"
    with open(search_json_path, "w", encoding="utf-8") as f:
        json.dump(search_index, f, indent=2)
    print(f"[+] Saved search index to: {search_json_path}")
    
    # Write assets/js/docs-bundle.js
    bundle_js_path = assets_js / "docs-bundle.js"
    bundle_content = f"""/**
 * PortProbe Enterprise Documentation Static Data Bundle
 * Automatically generated by scripts/generate_doc_index.py.
 * Do not edit manually. Re-run build script to update.
 */
window.PORTPROBE_VERSION = "2.0.0";
window.PORTPROBE_NAVIGATION = {json.dumps(navigation, indent=2)};
window.PORTPROBE_SEARCH_INDEX = {json.dumps(search_index, indent=2)};
window.PORTPROBE_DOCS = {json.dumps(docs_map, indent=2)};
"""
    with open(bundle_js_path, "w", encoding="utf-8") as f:
        f.write(bundle_content)
    print(f"[+] Generated zero-dependency docs bundle: {bundle_js_path}")
    print("[*] Documentation build complete successfully!")

if __name__ == "__main__":
    build_documentation_bundle()
