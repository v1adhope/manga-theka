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
- Docker Compose with `postgres` and `rustfs` for local development.
- Configuration from environment variables prefixed with `APP_`, listed in
  `.env.example`.
- Migrations for `creators`, `content_ratings`, `languages`, `books`, `labels`,
  `book_labels`, `book_links`, `book_titles`, `book_creators`, and
  `book_covers`.
- `/creators` and `/creators/{id}` for creator CRUD, listed page by page with a
  cursor.
- `/content-ratings`, `/languages`, and `/labels` to read the enum catalogs.
- `/books` and `/books/{id}` for book CRUD, carrying labels, links, alternative
  titles, and creators in the book payload.
- `/books/{id}/covers` to upload, list, and delete a book's cover images, and
  `/books/{id}/main-cover` to pick the one that represents it. An upload is a
  raw image body of at most 5 MiB, accepted only as JPEG, PNG, or WebP.
- `/books/{id}/covers/{coverId}/image` to fetch a cover image, redirecting to a
  short-lived link to the object storage that holds it.

[Unreleased]: https://github.com/v1adhope/manga-theka/commits/main
