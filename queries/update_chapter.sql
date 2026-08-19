update chapters
set number = $3, name = $4, volume = $5, updated_at = $6
where id = $1 and book_id = $2
returning id;
