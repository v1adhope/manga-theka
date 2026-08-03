# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/2.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `README.md`.
- `taskfile.yml` with all used commands.
- Telemetry. Tracing with logger support.
- Graceful shutdown with Ctrl+C and terminate signals.
- `/healthz` route to check service health.
- Docker Compose with `postgres` for local development.
- Migrations for `creator_roles`, `creators`, `content_ratings`, `book_statuses`,
  `images`, `book_types`, `languages`, `books`, `label_types`, `labels`.
- Configuration via `.env`.
- `/creators` and `/creators/{id}` for base creators usage.
