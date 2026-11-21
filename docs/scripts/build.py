#!/usr/bin/env python3
"""
PortProbe Documentation Site Build Script.
Runs index generation, content synchronization, and validates documentation links.
"""

import sys
import subprocess
from pathlib import Path

def main():
    script_dir = Path(__file__).resolve().parent
    gen_script = script_dir / "generate_doc_index.py"
    
    print("==================================================")
    print("      PORTPROBE DOCUMENTATION SITE BUILDER        ")
    print("==================================================")
    
    res = subprocess.run([sys.executable, str(gen_script)], check=True)
    if res.returncode == 0:
        print("[✓] Build completed successfully.")
        print("[*] To test locally:")
        print("    cd docs")
        print("    python -m http.server 8000")
        print("    Open http://localhost:8000 in your browser.")
    else:
        print("[✗] Build failed.")
        sys.exit(1)

if __name__ == "__main__":
    main()
