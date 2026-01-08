#!/bin/bash
# Monitor mesh generation progress

OUTPUT_DIR="output_parallel"
TOTAL=619

while true; do
    # Count completed meshes (directories with .glb files)
    COMPLETED=$(find "$OUTPUT_DIR" -name "*.glb" 2>/dev/null | wc -l | tr -d ' ')

    # Count in-progress (directories without .glb)
    DIRS=$(find "$OUTPUT_DIR" -maxdepth 1 -type d 2>/dev/null | wc -l | tr -d ' ')
    DIRS=$((DIRS - 1))  # Subtract the output_parallel dir itself

    # Calculate percentage
    if [ "$TOTAL" -gt 0 ]; then
        PCT=$((COMPLETED * 100 / TOTAL))
    else
        PCT=0
    fi

    # Get running process info
    PROCS=$(ps aux | grep "generate_meshes_parallel" | grep -v grep | wc -l | tr -d ' ')

    # Clear and print status
    clear
    echo "======================================================"
    echo "  PID-VLA Mesh Generation Monitor"
    echo "======================================================"
    echo "  Completed:    $COMPLETED / $TOTAL ($PCT%)"
    echo "  Directories:  $DIRS"
    echo "  Processes:    $PROCS"
    echo "======================================================"
    echo ""
    echo "Recent files:"
    ls -lt "$OUTPUT_DIR"/*/prompt.txt 2>/dev/null | head -10 | awk '{print "  " $NF}'
    echo ""
    echo "Press Ctrl+C to exit monitor (generation continues)"

    sleep 10
done
