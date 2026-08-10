select id, name, description, publication_year, content_rating, status, kind,
       publication_language, author, artist, updated_at, created_at
from books
where id = $1;
