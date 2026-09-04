update books
set name = $2, description = $3, publication_year = $4, content_rating = $5, status = $6,
    kind = $7, publication_language = $8, publication_demographic = $9, updated_at = $10
where id = $1
returning id;
