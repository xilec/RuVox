#!/usr/bin/env bash
# Test runner script for fast_tts
# Usage: ./scripts/test.sh [command] [args]

set -e

cd "$(dirname "$0")/.."

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

show_help() {
    echo -e "${BLUE}Fast TTS Test Runner${NC}"
    echo ""
    echo "Usage: ./scripts/test.sh [command] [args]"
    echo ""
    echo "Commands:"
    echo "  all             Run all tests (default)"
    echo "  quick           Run tests with short output (-q)"
    echo "  verbose         Run tests with verbose output (-v)"
    echo "  failed          Run only previously failed tests"
    echo "  stop            Stop on first failure (-x)"
    echo "  watch           Run tests in watch mode (requires pytest-watch)"
    echo ""
    echo "  module <name>   Run tests for specific module:"
    echo "                    english, abbreviations, numbers, urls, symbols, code, pipeline"
    echo ""
    echo "  match <pattern> Run tests matching pattern (-k)"
    echo "  cov             Run with coverage report"
    echo "  cov-html        Run with HTML coverage report"
    echo ""
    echo "  list            List all available tests"
    echo "  count           Count tests by module"
    echo "  passed          Show only passed tests from last run"
    echo ""
    echo "Examples:"
    echo "  ./scripts/test.sh                     # Run all tests"
    echo "  ./scripts/test.sh module numbers      # Run number tests only"
    echo "  ./scripts/test.sh match 'integer'     # Run tests matching 'integer'"
    echo "  ./scripts/test.sh stop                # Stop on first failure"
    echo "  ./scripts/test.sh cov                 # Run with coverage"
}

run_pytest() {
    uv run pytest "$@"
}

case "${1:-all}" in
    all)
        echo -e "${BLUE}Running all tests...${NC}"
        run_pytest --tb=short "${@:2}"
        ;;
    quick|-q)
        echo -e "${BLUE}Running tests (quick mode)...${NC}"
        run_pytest -q --tb=line "${@:2}"
        ;;
    verbose|-v)
        echo -e "${BLUE}Running tests (verbose)...${NC}"
        run_pytest -v --tb=short "${@:2}"
        ;;
    failed|--lf)
        echo -e "${BLUE}Running failed tests only...${NC}"
        run_pytest --lf --tb=short "${@:2}"
        ;;
    stop|-x)
        echo -e "${BLUE}Running tests (stop on first failure)...${NC}"
        run_pytest -x --tb=short "${@:2}"
        ;;
    watch|-w)
        echo -e "${BLUE}Running tests in watch mode...${NC}"
        uv run pytest-watch -- --tb=short "${@:2}"
        ;;
    module|-m)
        if [ -z "$2" ]; then
            echo -e "${RED}Error: module name required${NC}"
            echo "Available modules: english, abbreviations, numbers, urls, symbols, code, pipeline"
            exit 1
        fi
        MODULE="$2"
        case "$MODULE" in
            eng|english)
                FILE="tests/test_english.py"
                ;;
            abbr|abbreviations)
                FILE="tests/test_abbreviations.py"
                ;;
            num|numbers)
                FILE="tests/test_numbers.py"
                ;;
            url|urls)
                FILE="tests/test_urls.py"
                ;;
            sym|symbols)
                FILE="tests/test_symbols.py"
                ;;
            code)
                FILE="tests/test_code.py"
                ;;
            pipe|pipeline)
                FILE="tests/test_pipeline.py"
                ;;
            *)
                FILE="tests/test_${MODULE}.py"
                ;;
        esac
        echo -e "${BLUE}Running tests for module: ${MODULE}${NC}"
        run_pytest "$FILE" --tb=short "${@:3}"
        ;;
    match|-k)
        if [ -z "$2" ]; then
            echo -e "${RED}Error: pattern required${NC}"
            exit 1
        fi
        echo -e "${BLUE}Running tests matching: $2${NC}"
        run_pytest -k "$2" --tb=short "${@:3}"
        ;;
    cov|coverage)
        echo -e "${BLUE}Running tests with coverage...${NC}"
        run_pytest --cov=fast_tts --cov-report=term-missing "${@:2}"
        ;;
    cov-html)
        echo -e "${BLUE}Running tests with HTML coverage report...${NC}"
        run_pytest --cov=fast_tts --cov-report=html "${@:2}"
        echo -e "${GREEN}Coverage report: htmlcov/index.html${NC}"
        ;;
    list)
        echo -e "${BLUE}Listing all tests...${NC}"
        run_pytest --collect-only -q "${@:2}"
        ;;
    count)
        echo -e "${BLUE}Test count by module:${NC}"
        echo ""
        for file in tests/test_*.py; do
            name=$(basename "$file" .py | sed 's/test_//')
            count=$(run_pytest "$file" --collect-only -q 2>/dev/null | tail -1 | grep -oE '[0-9]+' | head -1 || echo "0")
            printf "  %-20s %s tests\n" "$name" "$count"
        done
        echo ""
        total=$(run_pytest --collect-only -q 2>/dev/null | tail -1 | grep -oE '[0-9]+' | head -1 || echo "0")
        echo -e "  ${GREEN}TOTAL: $total tests${NC}"
        ;;
    passed)
        echo -e "${BLUE}Showing passed tests from last run...${NC}"
        run_pytest --tb=no -v "${@:2}" 2>&1 | grep "PASSED" || echo "No passed tests found"
        ;;
    help|-h|--help)
        show_help
        ;;
    *)
        # Pass through to pytest
        run_pytest "$@"
        ;;
esac
