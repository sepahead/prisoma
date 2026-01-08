#!/usr/bin/env python3
"""Quick progress checker for mesh generation."""

import os
import time
from pathlib import Path
from datetime import datetime

def count_glb_files(directory):
    """Count .glb files in directory."""
    path = Path(directory)
    if not path.exists():
        return 0
    return len(list(path.glob("**/*.glb")))

def check_processes():
    """Check if generation processes are running."""
    import subprocess
    result = subprocess.run(
        ["pgrep", "-f", "generate_meshes_parallel"],
        capture_output=True, text=True
    )
    return len(result.stdout.strip().split('\n')) if result.stdout.strip() else 0

def main():
    main_dir = "output_parallel"
    supp_dir = "output_parallel_supplement"

    main_total = 619
    supp_total = 322

    start_time = time.time()

    print("\n" + "="*60)
    print("  PID-VLA Mesh Generation Progress")
    print("="*60)

    while True:
        main_count = count_glb_files(main_dir)
        supp_count = count_glb_files(supp_dir)
        total = main_count + supp_count
        grand_total = main_total + supp_total

        elapsed = time.time() - start_time
        rate = total / elapsed if elapsed > 0 else 0
        remaining = grand_total - total
        eta = remaining / rate / 60 if rate > 0 else 0

        procs = check_processes()

        os.system('clear')
        print("\n" + "="*60)
        print("  PID-VLA Mesh Generation Progress")
        print("="*60)
        print(f"  Time: {datetime.now().strftime('%H:%M:%S')}")
        print(f"  Elapsed: {elapsed/60:.1f} min")
        print("-"*60)
        print(f"  Main batch:    {main_count:4d} / {main_total} ({100*main_count/main_total:.1f}%)")
        print(f"  Supplement:    {supp_count:4d} / {supp_total} ({100*supp_count/supp_total:.1f}%)")
        print("-"*60)
        print(f"  TOTAL:         {total:4d} / {grand_total} ({100*total/grand_total:.1f}%)")
        print(f"  Rate:          {rate*60:.1f} meshes/min")
        print(f"  ETA:           {eta:.1f} min")
        print(f"  Processes:     {procs}")
        print("="*60)
        print("\n  Press Ctrl+C to exit (generation continues)")

        if procs == 0 and total > 0:
            print("\n  Generation appears complete!")
            break

        time.sleep(10)

if __name__ == "__main__":
    main()
