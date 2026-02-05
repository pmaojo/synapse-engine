#!/bin/bash

# Script to extract all Rust source code to a single text file
OUTPUT_FILE="codebase_extract.txt"

echo "Extracting codebase to $OUTPUT_FILE..."
echo "========================================" > "$OUTPUT_FILE"
echo "SYNAPSE CODEBASE EXTRACTION" >> "$OUTPUT_FILE"
echo "Generated: $(date)" >> "$OUTPUT_FILE"
echo "========================================" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# Find all Rust files and concatenate them
find crates -name "*.rs" -type f | sort | while read -r file; do
    echo "" >> "$OUTPUT_FILE"
    echo "========================================" >> "$OUTPUT_FILE"
    echo "FILE: $file" >> "$OUTPUT_FILE"
    echo "========================================" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    cat "$file" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
done

# Also include proto files
find crates -name "*.proto" -type f | sort | while read -r file; do
    echo "" >> "$OUTPUT_FILE"
    echo "========================================" >> "$OUTPUT_FILE"
    echo "FILE: $file" >> "$OUTPUT_FILE"
    echo "========================================" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    cat "$file" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
done

echo "Extraction complete! Output saved to $OUTPUT_FILE"
echo "Total lines: $(wc -l < "$OUTPUT_FILE")"
