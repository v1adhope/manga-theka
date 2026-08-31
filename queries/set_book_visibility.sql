update books
set visibility = $2, note = $3, submitted_at = coalesce($4, submitted_at)
where id = $1 and visibility = $5
returning id;
