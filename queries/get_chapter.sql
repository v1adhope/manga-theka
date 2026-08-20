select id, book_id, number, name, volume, updated_at, created_at
from chapters
where id = $1;
