#!/bin/bash
# shellcheck shell=bash

# Portable sorting functions for cross-platform compatibility
# Provides drop-in replacements for GNU sort -V (version sort) and -r (reverse)
#
# Known limitations compared to GNU sort -V:
# - Space handling: Our algorithm trims leading/trailing spaces, GNU preserves them with complex tie-breaking
# - Negative version numbers: GNU uses 3-tier priority system (valid > non-version > invalid patterns)  
# - These edge cases don't affect common patterns like app-1.0-linux or release-1.2
# - Our trimming approach is simpler and more predictable for practical usage
#
# sort_v: Portable version sort (replacement for sort -V)
# Sorts input lines by version number, supporting semantic versioning with prefix robustness
# Usage: sort_v < input_file
#        echo -e "1.2.3\n1.2.10\n1.2.2" | sort_v
sort_v() {
  # GNU sort -V compatible sorting with version specificity handling - enhanced error handling
  awk 'BEGIN {
    # Initialize variables to prevent undefined behavior in different awk implementations
    RSTART = 0; RLENGTH = 0
  }
  {
    line = $0
    original = $0
    # Trim leading and trailing spaces for better compatibility
    gsub(/^[ \t]+|[ \t]+$/, "", line)
    
    # Handle empty lines - they should sort first like GNU sort -V
    if (length(line) == 0) {
      print " |" original  # space sorts before all numbers/letters
      next
    }
    
    # GNU sort -V algorithm: iterate through string, extend all incomplete numbers
    # 1 -> 1.0, x.y -> x.y.0 (if not followed by .z)
    # This handles numbers anywhere in the string, not just at the end
    
    result_line = ""
    remaining = line
    
    while (length(remaining) > 0) {
      if (match(remaining, /[0-9]+(\.[0-9]+)*/)) {
        # Found a number or number sequence
        before = substr(remaining, 1, RSTART-1)
        number_part = substr(remaining, RSTART, RLENGTH)
        after_pos = RSTART + RLENGTH
        
        # Extend every number to x.y.z format for GNU sort -V compatibility
        # Count existing dots to determine what to append
        dot_count = gsub(/\./, ".", number_part)
        
        if (dot_count == 0) {
          # Single number: 1 -> 1.0.0
          number_part = number_part ".0.0"
        } else if (dot_count == 1) {
          # Two-part number: 1.2 -> 1.2.0
          number_part = number_part ".0"
        }
        # If dot_count >= 2, its already x.y.z+ format, leave as is
        
        result_line = result_line before number_part
        remaining = substr(remaining, after_pos)
      } else {
        # No more numbers, append the rest
        result_line = result_line remaining
        break
      }
    }
    
    line = result_line
    
    # Now pad all numbers for proper lexicographic sorting
    result = ""
    while (match(line, /[0-9]+/)) {
      result = result substr(line, 1, RSTART-1)
      number = substr(line, RSTART, RLENGTH)  
      result = result sprintf("%010d", number)
      line = substr(line, RSTART + RLENGTH)
    }
    result = result line
    
    # Base versions (1.2.3) should sort before pre-release versions (1.2.3-alpha)
    original = $0
    if (match(original, /^[^0-9]*[0-9]+([.][0-9]+)*$/)) {
      result = result "#"  # hash sorts before hyphen
    }
    
    print result "|" $0
  }
  ' | LC_ALL=C sort | cut -d'|' -f2-
}
export -f sort_v

# sort_rv: Portable reverse version sort (replacement for sort -V -r)  
# Sorts input lines by version number in descending order with prefix robustness
# Usage: sort_rv < input_file
#        echo -e "1.2.3\n1.2.10\n1.2.2" | sort_rv
sort_rv() {
  # GNU sort -V compatible reverse sorting - pure POSIX
  awk '
  {
    line = $0
    original = $0
    # Trim leading and trailing spaces for better compatibility
    gsub(/^[ \t]+|[ \t]+$/, "", line)
    
    # Handle empty lines - they should sort first like GNU sort -V (last in reverse)
    if (length(line) == 0) {
      print " |" original  # space sorts before all numbers/letters
      next
    }
    
    # GNU sort -V algorithm: iterate through string, extend all incomplete numbers
    # 1 -> 1.0, x.y -> x.y.0 (if not followed by .z)
    # This handles numbers anywhere in the string, not just at the end
    
    result_line = ""
    remaining = line
    
    while (length(remaining) > 0) {
      if (match(remaining, /[0-9]+(\.[0-9]+)*/)) {
        # Found a number or number sequence
        before = substr(remaining, 1, RSTART-1)
        number_part = substr(remaining, RSTART, RLENGTH)
        after_pos = RSTART + RLENGTH
        
        # Extend every number to x.y.z format for GNU sort -V compatibility
        # Count existing dots to determine what to append
        dot_count = gsub(/\./, ".", number_part)
        
        if (dot_count == 0) {
          # Single number: 1 -> 1.0.0
          number_part = number_part ".0.0"
        } else if (dot_count == 1) {
          # Two-part number: 1.2 -> 1.2.0
          number_part = number_part ".0"
        }
        # If dot_count >= 2, its already x.y.z+ format, leave as is
        
        result_line = result_line before number_part
        remaining = substr(remaining, after_pos)
      } else {
        # No more numbers, append the rest
        result_line = result_line remaining
        break
      }
    }
    
    line = result_line
    
    # Now pad all numbers for proper lexicographic sorting
    result = ""
    while (match(line, /[0-9]+/)) {
      result = result substr(line, 1, RSTART-1)
      number = substr(line, RSTART, RLENGTH)  
      result = result sprintf("%010d", number)
      line = substr(line, RSTART + RLENGTH)
    }
    result = result line
    
    # Base versions (1.2.3) should sort before pre-release versions (1.2.3-alpha)
    original = $0
    if (match(original, /^[^0-9]*[0-9]+([.][0-9]+)*$/)) {
      result = result "#"  # hash sorts before hyphen
    }
    
    print result "|" $0
  }
  ' | LC_ALL=C sort -r | cut -d'|' -f2-
}
export -f sort_rv

# version_gte: Check if version1 >= version2
# Returns 0 (true) if version1 is greater than or equal to version2
# Returns 1 (false) otherwise
# Usage: version_gte "1.2.3" "1.2.1" && echo "yes" || echo "no"
version_gte() {
  local version1="$1"
  local version2="$2"
  
  # If versions are identical, return true
  if [ "$version1" = "$version2" ]; then
    return 0
  fi
  
  # Sort both versions and check if version2 comes first
  # If version2 is the first line, then version1 >= version2
  local first_version
  first_version=$(printf '%s\n%s' "$version1" "$version2" | sort_v | head -n1)
  [ "$first_version" = "$version2" ]
}
export -f version_gte

# version_gt: Check if version1 > version2  
# Returns 0 (true) if version1 is strictly greater than version2
# Returns 1 (false) otherwise
# Usage: version_gt "1.2.3" "1.2.1" && echo "yes" || echo "no"
version_gt() {
  local version1="$1"
  local version2="$2"
  
  # If versions are identical, return false
  if [ "$version1" = "$version2" ]; then
    return 1
  fi
  
  # Use version_gte logic
  version_gte "$version1" "$version2"
}
export -f version_gt

# version_lte: Check if version1 <= version2
# Returns 0 (true) if version1 is less than or equal to version2  
# Returns 1 (false) otherwise
# Usage: version_lte "1.2.1" "1.2.3" && echo "yes" || echo "no"
version_lte() {
  local version1="$1" 
  local version2="$2"
  version_gte "$version2" "$version1"
}
export -f version_lte

# version_lt: Check if version1 < version2
# Returns 0 (true) if version1 is strictly less than version2
# Returns 1 (false) otherwise  
# Usage: version_lt "1.2.1" "1.2.3" && echo "yes" || echo "no"
version_lt() {
  local version1="$1"
  local version2="$2"
  version_gt "$version2" "$version1"  
}
export -f version_lt