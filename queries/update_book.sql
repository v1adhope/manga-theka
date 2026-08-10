update books
set name = $2, description = $3, publication_year = $4, content_rating = $5, status = $6,
    kind = $7, publication_language = $8, author = $9, artist = $10, updated_at = $11
where id = $1
returning id;
