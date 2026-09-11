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
- Docker Compose with `postgres`, `rustfs`, and `redis` for local
  development.
- Configuration from environment variables prefixed with `APP_`, listed in
  `.env.example`.
- Migrations for `users`, `creators`, `content_ratings`, `languages`,
  `books`, `labels`, `book_labels`, `book_links`, `book_titles`,
  `book_creators`, `book_covers`, `chapters`, `chapter_localizations`,
  `chapter_releases`, `chapter_pages`, and `feedback`.
- `/users/register` and `/users/me`, plus `/sessions/login`,
  `/sessions/refresh`, `/sessions/me`, `/sessions/me/current`,
  `/sessions/me/all`, and `/sessions/me/{sid}` for account registration and
  a JWT-backed session lifecycle: Argon2id-hashed passwords, dual
  Ed25519-signed access/refresh tokens, refresh sessions tracked in Redis.
  Roles rank `Reader < Uploader < Moderator < Admin` and gate routes
  accordingly.
- `/creators` and `/creators/{id}` for creator CRUD, listed page by page
  with a cursor. Role lives on the per-book credit, not the creator
  identity; reads return it as an aggregated `roles` array.
- `/content-ratings`, `/languages`, and `/labels` to read the enum
  catalogs. Label `kind` is `Genre`, `Theme`, or `Presentation`.
- `/books` and `/books/{id}` for book CRUD, carrying labels, links,
  alternative titles, and per-book creator credits (`creatorId` + `role`)
  in the payload. `POST /books` always creates a `Draft`;
  `PUT /books/{id}/visibility` is the sole guarded transition to
  `PendingReview`, `Listed`, `Rejected`, or `Hidden`. `GET /books` filters
  by content rating, publication language, publication demographic,
  status, labels (AND/OR, with exclusion), and available translated
  language; sorts by creation time, name, or publication year; and pages
  through an opaque cursor that 400s if the filter or sort changes
  mid-page.
- `/books/{id}/covers` to upload and list a book's cover images,
  `DELETE /covers/{id}` to remove one, and `/books/{id}/main-cover` to
  pick the one that represents it. An upload is a raw image body of at
  most 5 MiB, sniffed as JPEG, PNG, or WebP.
- `/covers/{id}/image` to fetch a cover image, redirecting to a
  short-lived link to the object storage that holds it.
- `/books/{id}/chapters` and `/chapters/{id}` for chapter CRUD, each
  chapter carrying per-language localizations that replace wholesale on
  write.
- `/chapters/{id}/releases` and `/releases/{id}` to create, list, read,
  and delete a chapter's releases; `/releases/{id}/upload` and
  `/releases/{id}/commit` to stage page images and then commit the whole
  ordered set in one call. Staging, committing, and deleting a release is
  gated to its creator, a Moderator, or an Admin.
- `/releases/{id}/pages` and `/releases/{id}/pages/{page_number}` to list
  a release's pages and fetch one page, redirecting to a freshly presigned
  storage URL.
- `/feedbacks`, `/feedbacks/{id}`, and `/feedbacks/{id}/status` for
  anonymous inbound Report, Correction, and General messages from guests,
  listed and resolved by moderators.
- Reads of book, cover, and release metadata stay visible while a book is
  `Listed` or `Hidden`; page and cover bytes require `Listed` (or a
  Moderator/Admin caller).

[Unreleased]: https://github.com/v1adhope/manga-theka/commits/main
