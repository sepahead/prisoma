#!/bin/bash
for i in {1..10}
do
   LOG_FILE="meshmaker/spawn_swarm_$i.log"
   echo "Launching Batch $i -> $LOG_FILE"
   nohup python3 meshmaker/parallel_spawn_industrial.py > "$LOG_FILE" 2>&1 &
   PID=$!
   echo "Spawned PID: $PID"
   sleep 2
done
