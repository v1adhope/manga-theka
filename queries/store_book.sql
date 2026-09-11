insert into books(id, name, description, publication_year, content_rating, status, kind,
                  publication_language, publication_demographic, visibility, updated_at,
                  created_at, created_by)
values($1, $2, $3, $4, $5, $6, $7, $8, $9, 'Draft', $10, $11, $12);
