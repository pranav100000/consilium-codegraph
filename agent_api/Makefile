.PHONY: test test-unit test-integration test-coverage clean install lint format type-check

# Install dependencies
install:
	pip install -r requirements.txt
	pip install -e .

# Run all tests
test:
	pytest

# Run only unit tests
test-unit:
	pytest -m "unit or not integration"

# Run only integration tests  
test-integration:
	pytest -m integration

# Run tests with coverage
test-coverage:
	pytest --cov=agent_api --cov-report=html --cov-report=term

# Clean up
clean:
	find . -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
	find . -type f -name "*.pyc" -delete
	rm -rf .pytest_cache
	rm -rf htmlcov
	rm -rf .coverage
	rm -rf *.egg-info
	rm -rf build dist

# Linting
lint:
	flake8 agent_api tests
	black --check agent_api tests
	isort --check-only agent_api tests

# Format code
format:
	black agent_api tests
	isort agent_api tests

# Type checking
type-check:
	mypy agent_api

# Run all checks
check: lint type-check test

# Development setup
dev: install
	@echo "Development environment ready!"

# Help
help:
	@echo "Available commands:"
	@echo "  make install        - Install dependencies"
	@echo "  make test          - Run all tests"
	@echo "  make test-unit     - Run unit tests only"
	@echo "  make test-coverage - Run tests with coverage report"
	@echo "  make lint          - Run linting checks"
	@echo "  make format        - Format code with black and isort"
	@echo "  make type-check    - Run mypy type checking"
	@echo "  make clean         - Clean up generated files"
	@echo "  make check         - Run all checks (lint, type-check, test)"
	@echo "  make dev           - Set up development environment"