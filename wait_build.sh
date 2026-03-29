#!/bin/bash
while true; do
  if ps aux | grep -v grep | grep 'cargo build' > /dev/null; then
    sleep 10
  else
    break
  fi
done
echo "Build finished"
