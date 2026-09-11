update books
set visibility = $2, note = $3, submitted_at = coalesce($4, submitted_at), updated_at = $5
where id = $1 and visibility = $6
returning id;
