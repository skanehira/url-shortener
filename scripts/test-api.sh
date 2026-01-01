#!/usr/bin/env bash
set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

SHORTENER_URL="${SHORTENER_URL:-http://localhost:8080}"
ANALYTICS_URL="${ANALYTICS_URL:-http://localhost:8081}"

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_response() {
    local name="$1"
    local response="$2"
    local expected="$3"

    if echo "$response" | grep -q "$expected"; then
        info "$name: OK"
        return 0
    else
        error "$name: FAILED"
        echo "  Response: $response"
        return 1
    fi
}

wait_for_service() {
    local url="$1"
    local name="$2"
    local max_attempts=30
    local attempt=1

    info "Waiting for $name to be ready..."
    while [ $attempt -le $max_attempts ]; do
        if curl -sf "$url/health" > /dev/null 2>&1; then
            info "$name is ready"
            return 0
        fi
        echo -n "."
        sleep 1
        attempt=$((attempt + 1))
    done
    echo
    error "$name did not become ready in time"
    return 1
}

main() {
    echo "========================================"
    echo " URL Shortener API Test"
    echo "========================================"
    echo

    # Wait for services
    wait_for_service "$SHORTENER_URL" "shortener-service"
    wait_for_service "$ANALYTICS_URL" "analytics-service"
    echo

    # Test 1: Health check
    info "Test 1: Health check"
    response=$(curl -sf "$SHORTENER_URL/health")
    check_response "  shortener-service /health" "$response" '"status":"ok"'

    response=$(curl -sf "$ANALYTICS_URL/health")
    check_response "  analytics-service /health" "$response" '"status":"ok"'
    echo

    # Test 2: Readiness check
    info "Test 2: Readiness check"
    response=$(curl -sf "$SHORTENER_URL/ready")
    check_response "  shortener-service /ready" "$response" '"status":"ready"'

    response=$(curl -sf "$ANALYTICS_URL/ready")
    check_response "  analytics-service /ready" "$response" '"status":"ready"'
    echo

    # Test 3: Create URL
    info "Test 3: Create short URL"
    response=$(curl -sf -X POST "$SHORTENER_URL/api/v1/urls" \
        -H "Content-Type: application/json" \
        -d '{"url": "https://example.com/test"}')
    check_response "  POST /api/v1/urls" "$response" '"original_url":"https://example.com/test"'

    code=$(echo "$response" | jq -r '.code')
    info "  Created short code: $code"
    echo

    # Test 4: Get URL
    info "Test 4: Get URL by code"
    response=$(curl -sf "$SHORTENER_URL/api/v1/urls/$code")
    check_response "  GET /api/v1/urls/$code" "$response" '"code":"'"$code"'"'
    echo

    # Test 5: List URLs
    info "Test 5: List URLs"
    response=$(curl -sf "$SHORTENER_URL/api/v1/urls")
    check_response "  GET /api/v1/urls" "$response" "$code"
    echo

    # Test 6: Redirect (check redirect response, not the target)
    info "Test 6: Redirect"
    http_code=$(curl -s -o /dev/null -w "%{http_code}" "$SHORTENER_URL/$code")
    if [ "$http_code" = "307" ]; then
        info "  GET /$code: OK (HTTP 307 Temporary Redirect)"
    elif [ "$http_code" = "301" ] || [ "$http_code" = "302" ]; then
        info "  GET /$code: OK (HTTP $http_code Redirect)"
    else
        error "  GET /$code: FAILED (expected redirect, got HTTP $http_code)"
    fi
    echo

    # Test 7: Wait for analytics to be processed
    info "Test 7: Analytics (waiting for event processing...)"
    sleep 2

    response=$(curl -sf "$ANALYTICS_URL/api/v1/analytics/$code" || echo '{"error": "not found"}')
    if echo "$response" | grep -q '"access_count"'; then
        access_count=$(echo "$response" | jq -r '.access_count')
        info "  GET /api/v1/analytics/$code: OK (access_count: $access_count)"
    else
        warn "  GET /api/v1/analytics/$code: Analytics not yet available (async processing)"
    fi
    echo

    # Test 8: List analytics
    info "Test 8: List analytics"
    response=$(curl -sf "$ANALYTICS_URL/api/v1/analytics")
    check_response "  GET /api/v1/analytics" "$response" '"items"'
    echo

    # Test 9: Update URL
    info "Test 9: Update URL"
    response=$(curl -sf -X PUT "$SHORTENER_URL/api/v1/urls/$code" \
        -H "Content-Type: application/json" \
        -d '{"url": "https://example.com/updated"}')
    check_response "  PUT /api/v1/urls/$code" "$response" '"original_url":"https://example.com/updated"'
    echo

    # Test 10: Delete URL
    info "Test 10: Delete URL"
    http_code=$(curl -sf -o /dev/null -w "%{http_code}" -X DELETE "$SHORTENER_URL/api/v1/urls/$code")
    if [ "$http_code" = "204" ]; then
        info "  DELETE /api/v1/urls/$code: OK"
    else
        error "  DELETE /api/v1/urls/$code: FAILED (HTTP $http_code)"
    fi
    echo

    echo "========================================"
    echo " All tests completed!"
    echo "========================================"
}

main "$@"
