insert into books(id, name, description, publication_year, content_rating, status, kind,
                  publication_language, author, artist, updated_at, created_at)
values($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12);
