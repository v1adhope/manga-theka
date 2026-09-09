select id, kind, status, email, note, book_id, updated_at, created_at
from feedback
where id = $1;
