# Roadmap

Monetization (check the law):
- [ ] Release statistics (time and day, how much was added to bookmarks)
- [ ] Ads
- [ ] Prepaid chapters and other one-time purchases

Backend:
- [ ] Deferred publication
- [ ] Flexible markdown news (posts), some activity (communities, achievements)
- [ ] Suggestions
- [ ] Export/import user data
- [ ] RSS (research; apply if there are suitable use cases)
- [ ] Lazy sweep for `orphaned_objects`
- [ ] Personal account can be converted to a group account
- [ ] Comments system
- [ ] Creator query has book refs
- [ ] Search by book name
- [ ] Email verification + neutral register message on email conflict
- [ ] Hide releases when necessary
- [ ] Add metrics with low cardinality first
- [ ] Lists for users

Frontend:
- [ ] Reading modes (scroll, paging)
- [ ] Jump by thumbnails to navigate between pages

By demand:
- [ ] Optimize WAL with PUT `/books` (when it hits performance issues)
- [ ] Optimize WAL with PUT `/chapters` (when it hits performance issues)
- [ ] Mock S3 instead of a real instance (when test time becomes a bottleneck)
- [ ] Optionally drop unused fields from query requests (when it hits a throughput bottleneck)
- [ ] Support uploading up to 10 covers at once (when it hits a throughput bottleneck)
- [ ] Anonymize user on delete (when user payment features arrive)
- [ ] Entity query expansion (X returns Y as extra)

## Known issues

- [ ] Consider what to do with CWE-367 (Time-of-Check to Time-of-Use / TOCTOU). Mitigate/Ignore/Fix
- [ ] JP and ENG warranty when writing books and chapters. Moderator perspective / Automation
- [ ] Feedback can include not only the book ID, but also chapters and releases. Include as note description / Extend current state
- [ ] Use a flexible string for chapter number / Keep current state
