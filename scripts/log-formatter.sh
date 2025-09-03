#!/bin/bash

# Log formatter script for LaTeX IDE Docker container
# This script prefixes logs with service names and timestamps for better readability

# Colors for different services
declare -A COLORS
COLORS[web-frontend]="\033[1;32m"      # Green
COLORS[webtransport-server]="\033[1;34m" # Blue  
COLORS[sse-handler]="\033[1;33m"       # Yellow
COLORS[model-manager]="\033[1;35m"     # Magenta
COLORS[latex-compiler]="\033[1;36m"    # Cyan
COLORS[supervisord]="\033[1;37m"       # White
RESET_COLOR="\033[0m"

# Function to format log line with service prefix and timestamp
format_log() {
    local service="$1"
    local line="$2"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    local color="${COLORS[$service]:-\033[1;37m}"
    
    echo -e "${color}[${timestamp}] [${service}]${RESET_COLOR} ${line}"
}

# Export the function so it can be used by supervisor
export -f format_log
export COLORS
export RESET_COLOR

# If script is called directly, read from stdin and format
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    service_name="${1:-unknown}"
    while IFS= read -r line; do
        format_log "$service_name" "$line"
    done
fi