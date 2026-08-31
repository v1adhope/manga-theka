insert into books(id, name, description, publication_year, content_rating, status, kind,
                  publication_language, visibility, updated_at, created_at)
values($1, $2, $3, $4, $5, $6, $7, $8, 'Draft', $9, $10);
