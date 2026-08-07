# Chapter pages served via short-lived presigned RustFS URLs behind a permanent app route

`GET /releases/{release_id}/pages/{page_number}` is a permanent, bookmarkable API route that 302-redirects to a freshly presigned RustFS URL computed at request time -- presigning is local HMAC signing against the storage access key, not a network round-trip to RustFS -- rather than either streaming image bytes through the app or exposing a public/unsigned bucket URL.

This was chosen over full byte-proxying to avoid routing every page-view's image bytes (up to 5MB each) through the app process on a self-hosted deployment. It was chosen over a public/unsigned bucket URL, or a bare presigned URL handed to the client directly, because a permanent reader link was a hard requirement ("live forever until deleted explicitly") that a bare presigned URL can't satisfy on its own -- its whole security property is the expiry. Wrapping the presign behind a stable app route gets both: the link the client holds never breaks, while the actual bytes are fetched directly from storage with no app bandwidth cost, since each redirect target is signed fresh.

Chapter pages have no auth gate today -- reads are anonymous per ADR-0003. This shape means adding one later is a change to the redirect handler (check authorization, then presign-and-redirect or refuse), not a URL-scheme migration.
