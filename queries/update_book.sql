update books
set name = $2, description = $3, publication_year = $4, content_rating = $5, status = $6,
    kind = $7, publication_language = $8, updated_at = $9
where id = $1
returning id;
